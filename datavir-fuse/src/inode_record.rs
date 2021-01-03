use crate::prelude::*;
#[allow(unused_imports)]
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use rusqlite::params;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

#[allow(unused_imports)]
use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

pub const INODE_NULL: u64 = 0;
pub const INODE_ROOT: u64 = 1;
pub const INODE_CONFIG: u64 = 2;
pub const INODE_SOCKET: u64 = 3;
pub const INODE_STATUS_FILE: u64 = 4;
pub const INODE_VOLUMES_DIR: u64 = 5;
pub const INODE_ALL_BUNDLES_DIR: u64 = 6;
pub const INODE_ALL_FILTERS_DIR: u64 = 7;
pub const INODE_TRASH_DIR: u64 = 8;

pub const INODE_MIN: i64 = 64;

#[allow(dead_code)]
const MIN_HARD_LINKS_DIR: u32 = 2;
#[allow(dead_code)]
static mut DEFAULT_DIR_PERM: u16 = 0o755;
#[allow(dead_code)]
static mut DEFAULT_FILE_PERM: u16 = 0o644;
#[allow(dead_code)]
static mut DEFAULT_UID: u32 = 0;
#[allow(dead_code)]
static mut DEFAULT_GID: u32 = 0;

const BLOCK_SIZE: u64 = 4096;
const DEFAULT_RDEV: u32 = 0; //no idea what it does
const DEFAULT_NODE_FLAGS: u32 = 0; //not sure what I should put here
const DEFAULT_NODE_PADDING: u32 = 0; //no idea what it does

pub trait INodeRegisterable: std::fmt::Debug {
    fn get_uuid(&self) -> Uuid;
    fn get_obj_type(&self) -> ObjectType;
    fn get_name(&self) -> String;
}

#[inline]
fn to_timestamp(time: SystemTime) -> Result<u64, SystemTimeError> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(v) => Ok(v.as_secs()),
        Err(err) => {
            error!(
                "Failed to convert SystemTime {:?} to UNIX timestamp: {:?}",
                time, err
            );
            Err(err)
        }
    }
}

#[inline]
fn time2sql(time: SystemTime) -> Result<i64, SystemTimeError> {
    let secs = to_timestamp(time)?;
    Ok(u64_to_i64(secs))
}

#[inline]
// can only return error TimeConversionErrorFromSecs
fn sql2time(time: i64) -> DVResult<SystemTime> {
    let secs = i64_to_u64(time);
    let duration = Duration::new(secs, 0);
    match UNIX_EPOCH.checked_add(duration) {
        None => {
            error!(
                "Failed to convert {} (interpreted as {} seconds since UNIX Epoch) to SystemTime",
                time, secs
            );
            Err(DVError::TimeConversionErrorFromSecs(secs))
        }
        Some(v) => Ok(v),
    }
}

fn string2file_type(val: &str) -> DVResult<FileType> {
    match val {
        "F" => Ok(FileType::RegularFile),
        "D" => Ok(FileType::Directory),
        "C" => Ok(FileType::CharDevice),
        "P" => Ok(FileType::NamedPipe),
        "B" => Ok(FileType::BlockDevice),
        "S" => Ok(FileType::Socket),
        "L" => Ok(FileType::Symlink),
        _ => Err(DVError::FuseTypeParseError(val.to_string())),
    }
}

fn file_type2string(kind: FileType) -> &'static str {
    match kind {
        FileType::RegularFile => "F",
        FileType::Directory => "D",
        FileType::CharDevice => "C",
        FileType::NamedPipe => "P",
        FileType::BlockDevice => "B",
        FileType::Socket => "S",
        FileType::Symlink => "L",
    }
}

#[derive(Debug, Clone)]
pub struct NodeName {
    inode: u64,
    parent: u64,
    /// This hidden IS NOT the `.` on the begining of filenames nor the NTFS hidden file attribute.
    hidden: bool,
    name: String,
    file_type: Option<FileType>,
}

impl NodeName {
    #[allow(dead_code)]
    pub fn new(inode: u64, parent: u64, hidden: bool, name: String) -> NodeName {
        NodeName {
            inode: inode,
            parent: parent,
            hidden: hidden,
            name: name,
            file_type: None,
        }
    }
    fn from_row(row: &rusqlite::Row) -> DVResult<NodeName> {
        Ok(NodeName {
            inode: i64_to_u64(row.get(0)?),
            parent: i64_to_u64(row.get(1)?),
            hidden: row.get(2)?,
            name: row.get(3)?,
            file_type: Some(string2file_type(&row.get::<usize, String>(4)?)?),
        })
    }
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
    fn get_inode_i64(&self) -> i64 {
        u64_to_i64(self.inode)
    }
    #[allow(dead_code)]
    pub fn get_parent(&self) -> u64 {
        self.parent
    }
    fn get_parent_i64(&self) -> i64 {
        u64_to_i64(self.parent)
    }
    #[allow(dead_code)]
    pub fn get_hidden(&self) -> bool {
        self.hidden
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_file_type(&self) -> Option<FileType> {
        self.file_type
    }
    #[allow(dead_code)]
    pub fn save(&self, db_mutex: &Mutex<()>, tx: &Transaction) -> DVResult<()> {
        let res;
        {
            let _v = db_mutex.lock().unwrap();
            res = tx.execute(
                "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
                (?, ?, ?, ?)",
                params![
                    self.get_inode_i64(),
                    self.get_parent_i64(),
                    self.hidden,
                    self.name
                ],
            );
        }
        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Failed to save NodeName ({:?}): {:?}", self, err);
                Err(DVError::SQLError(err))
            }
        }
    }
    pub fn find<'ans, 'db: 'ans>(
        parent: u64,
        include_hidden: bool,
        offset: Option<i64>,
        conn: &'db rusqlite::Connection
    ) -> DVResult<NodeNameIter<'ans>> {
        let mut sql = "SELECT `inode`, `parent`, `hidden`, `name`, `file_type` FROM `node_view` WHERE `parent` = ?1".to_string();
        if include_hidden == false {
            sql += " AND `hidden` = 0"
        }
        if let Some(offset) = offset {
            sql += &format!(" LIMIT -1 OFFSET {}", offset).to_string();
        }
        debug!("{}", &sql);
        let stmt = match conn.prepare(&sql) {
            Ok(v) => v,
            Err(err) => {
                error!(
                    "-NodeName::find(parent: {}, sql: {}) Failed to prepare statement: {:?}",
                    parent, sql, err
                );
                return Err(DVError::from(err));
            }
        };
        let mut ans = NodeNameIter {
            pos: 0,
            parent: parent,
            stmt: Box::new(stmt),
            rows: MaybeUninit::uninit(),
        };
        ans.rows = MaybeUninit::new(
            match fuck_mut(&mut ans.stmt).query(params![u64_to_i64(parent)]) {
                Ok(v) => v,
                Err(err) => {
                    error!(
                        "-NodeName::find(parent: {}, sql: {}) Failed to run statement: {:?}",
                        parent, sql, err
                    );
                    return Err(DVError::from(err));
                }
            },
        );
        Ok(ans)
    }

    #[allow(dead_code)]
    pub fn del(&self, tx: &Transaction) -> DVResult<()> {
        let res = tx.execute(
            "DELETE FROM `node_name` WHERE `inode` = ?1 AND `parent` = ?2 AND `name` = ?3",
            params![self.get_inode_i64(), self.get_parent_i64(), self.name],
        );
        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Failed to delete NodeName ({:?}): {:?}", self, err);
                Err(DVError::SQLError(err))
            }
        }
    }
}

pub struct NodeNameIter<'a> {
    pos: u64,
    parent: u64,
    stmt: Box<rusqlite::Statement<'a>>,
    rows: MaybeUninit<rusqlite::Rows<'a>>,
}

impl std::fmt::Debug for NodeNameIter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("NodeNameIter")
            .field("parent", &self.parent)
            .field("pos", &self.pos)
            .finish()
    }
}

impl<'a> NodeNameIter<'a> {
    #[allow(dead_code)]
    pub fn get_pos(&self) -> u64 {
        self.pos
    }
    #[allow(dead_code)]
    pub fn get_parent(&self) -> u64 {
        self.parent
    }
    fn get_rows(&mut self) -> &mut rusqlite::Rows<'a> {
        unsafe { &mut *self.rows.as_mut_ptr() }
    }

    pub fn try_next(&mut self) -> DVResult<NodeName> {
        let row = match self.get_rows().next() {
            Ok(v) => v,
            Err(err) => {
                error!(
                    "-NodeNameIter::try_next(self={:?}) Failed to get next row: {:?}",
                    self, err
                );
                return Err(DVError::from(err));
            }
        };
        if let Some(row) = row {
            let ans = NodeName::from_row(row);
            if ans.is_ok() {
                self.pos += 1;
            }
            ans
        } else {
            Err(DVError::NoMoreResults)
        }
    }
}

impl Iterator for NodeNameIter<'_> {
    type Item = NodeName;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let ans = self.try_next();
            match ans {
                Ok(name) => return Some(name),
                Err(err) => match err {
                    DVError::NoMoreResults => return None,
                    _ => {
                        error!("NodeNameIter::next({:?}): {:?}", self, err);
                    }
                },
            };
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeRecord {
    inode: u64,
    nlink: u32,
    obj_uuid: Uuid,
    obj_type: ObjectType,
    file_type: FileType,
    mtime: SystemTime,
    ctime: SystemTime,
    crtime: SystemTime,
    names: Option<Vec<NodeName>>,
}

impl NodeRecord {
    #[allow(dead_code)]
    pub fn get_inode(&self) -> u64 {
        self.inode
    }
    #[allow(dead_code)]
    pub fn get_obj_uuid(&self) -> Uuid {
        self.obj_uuid
    }
    #[allow(dead_code)]
    pub fn get_obj_type(&self) -> ObjectType {
        self.obj_type
    }
    #[allow(dead_code)]
    pub fn get_file_type(&self) -> FileType {
        self.file_type
    }
    #[allow(dead_code)]
    pub fn get_names(&self) -> Option<&Vec<NodeName>> {
        self.names.as_ref()
    }

    #[allow(dead_code)]
    pub fn can_readdir(&self, conn: &Connection) -> bool {
        if self.file_type == FileType::Directory {
            return true;
        }
        let mut list = match NodeName::find(self.inode, false, None, conn) {
            Ok(v) => v,
            Err(err) => {
                error!("{:?}", err);
                return false;
            }
        };
        return list.next().is_some();
    }

    fn new(
        num: u64,
        nlink: u32,
        uuid: Uuid,
        obj_type: ObjectType,
        file_type: FileType,
        mtime: SystemTime,
        ctime: SystemTime,
        crtime: SystemTime,
    ) -> Self {
        NodeRecord {
            inode: num,
            nlink: nlink,
            obj_uuid: uuid,
            obj_type: obj_type,
            file_type: file_type,
            mtime: mtime,
            ctime: ctime,
            crtime: crtime,
            names: None,
        }
    }

    fn from_row(row: &rusqlite::Row) -> DVResult<Self> {
        let inode = i64_to_u64(row.get(0)?);
        let mut nlink = row.get(1)?;
        let obj_uuid = parse_uuid(&row.get::<_, String>(2)?)?;
        let obj_type = row.get(3)?;
        let file_type = string2file_type(&row.get::<_, String>(4)?)?;
        let mtime = sql2time(row.get(5)?)?;
        let ctime = sql2time(row.get(6)?)?;
        let crtime = sql2time(row.get(7)?)?;

        if inode == INODE_ROOT {
            nlink = 2;
        }

        Ok(NodeRecord::new(
            inode, nlink, obj_uuid, obj_type, file_type, mtime, ctime, crtime,
        ))
    }

    #[allow(dead_code)]
    pub fn new_now(inode: u64, nlink: u32, uuid: Uuid, obj_type: ObjectType, file_type: FileType) -> Self {
        let now = SystemTime::now();
        NodeRecord::new(inode, nlink, uuid, obj_type, file_type, now, now, now)
    }

    fn get_inode_i64(&self) -> i64 {
        u64_to_i64(self.inode)
    }

    #[allow(dead_code)]
    pub fn save(&mut self, db_mutex: &Mutex<()>, tx: &Transaction) -> DVResult<()> {
        trace!("+NodeRecord::save(self={:?})", self);
        let now = SystemTime::now();
        let now_unix = time2sql(now)?;
        let had_inode = self.inode == INODE_NULL;
        let sql = match had_inode {
            true => "INSERT INTO `node_meta` (`obj_uuid`, `obj_type`, `file_type`, `ctime`, `mtime`, `crtime`) VALUES \
                    (?, ?, ?, ?, ?, ?)",
            false => "REPLACE INTO `node_meta` (`inode`, `obj_uuid`, `obj_type`, `file_type`, `ctime`) VALUES \
                    (?, ?, ?, ?, ?)",
        };
        let file_type_str = file_type2string(self.file_type);
        let inode = self.get_inode_i64();
        let params: Vec<&dyn rusqlite::ToSql>;
        params = match had_inode {
            true => vec![
                &self.obj_uuid,
                &self.obj_type,
                &file_type_str,
                &now_unix,
                &now_unix,
                &now_unix,
            ],
            false => vec![
                &inode,
                &self.obj_uuid,
                &self.obj_type,
                &file_type_str,
                &now_unix,
            ],
        };
        let res;
        let new_inode;
        {
            let _v = db_mutex.lock().unwrap();
            res = tx.execute(sql, params);
            new_inode = tx.last_insert_rowid();
        }
        if let Err(err) = res {
            error!("Failed to update metadata for NodeRecord: {:?}", err);
            trace!("-NodeRecord::save(self={:?}) -> {:?}", self, err);
            return Err(DVError::SQLError(err));
        }
        self.ctime = now;
        if !had_inode {
            self.mtime = now;
            self.crtime = now;
            self.inode = i64_to_u64(new_inode);
        }
        trace!("-NodeRecord::save(self={:?}) -> Ok", self);
        Ok(())
    }

    #[allow(dead_code)]
    // should be called when the file contents are changed
    pub fn update_mtime(&mut self, tx: &Transaction) -> DVResult<()> {
        trace!("+NodeRecord::update_mtime(self={:?})", self);
        let inode = self.get_inode_i64();
        if inode == 0 {
            trace!("-NodeRecord::update_mtime(self={:?}) -> NodeNoNum", self);
            return Err(DVError::NodeNoNum);
        }
        let now = SystemTime::now();
        let now_unix = time2sql(now)?;
        let res = tx.execute(
            "UPDATE `inode` SET `mtime` = ?2 WHERE `inode` = ?1",
            params![inode, now_unix],
        );
        if let Err(err) = res {
            error!("Failed to update mtime for NodeRecord: {:?}", err);
            trace!("-NodeRecord::update_mtime(self={:?}) -> {:?}", self, err);
            Err(DVError::SQLError(err))
        } else {
            self.mtime = now;
            debug!("Updated mtime for {:?}", self);
            trace!("-NodeRecord::update_mtime(self={:?}) -> Ok", self);
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn search(
        parent: Option<u64>,
        name: Option<&str>,
        obj_type: Option<ObjectType>,
        obj_uuid: Option<Uuid>,
        file_type: Option<FileType>,
        limit: Option<u64>,
        offset: Option<i64>,
        tx: &Transaction,
    ) -> DVResult<Vec<NodeRecord>> {
        let trace_msg = format!("NodeRecord::search(parent={:?}, name={:?}, obj_type={:?}, obj_uuid={:?}, file_type={:?}", parent, name, obj_type, obj_uuid, file_type);
        trace!("+{}", trace_msg);
        let mut sql = "SELECT `inode`, `parent`, `obj_uuid`, `obj_type`, `file_type`, `name`, `mtime`, `ctime`, `crtime` FROM `node_meta` WHERE 1 = 1".to_string();
        let file_type2 = match file_type {
            Some(s) => file_type2string(s),
            None => "-",
        };
        let parent = match parent {
            Some(n) => Some(u64_to_i64(n)),
            None => None,
        };
        let mut params = Vec::<&dyn rusqlite::ToSql>::new();
        if parent.is_some() {
            params.push(&parent);
            sql += &format!(" AND `parent` = ?{}", params.len());
        }
        if name.is_some() {
            params.push(&name);
            sql += &format!(" AND `name` = ?{}", params.len());
        } else {
            sql += " AND `name` IS NOT NULL";
        }
        if obj_type.is_some() {
            params.push(&obj_type);
            sql += &format!(" AND `obj_type` = ?{}", params.len());
        }
        if obj_uuid.is_some() {
            params.push(&obj_uuid);
            sql += &format!(" AND `obj_uuid` = ?{}", params.len());
        }
        if file_type.is_some() {
            params.push(&file_type2);
            sql += &format!(" AND `file_type` = ?{}", params.len());
        }
        if offset.is_some() {
            sql += " ORDER BY `inode` ASC";
        }
        if let Some(limit) = limit {
            sql += &format!(" LIMIT {}", limit);
        }
        if let Some(offset) = offset {
            // SQLite dosen't like OFFSET without LIMIT
            if limit.is_some() {
                sql += &format!(" OFFSET {}", offset);
            }
        }
        debug!("{}", sql);
        let mut stmt = tx.prepare(&sql)?;
        let mut rows = stmt.query(params)?;
        let mut ans = Vec::new();
        let mut last_err: Option<DVError> = None;

        while let Some(row) = rows.next()? {
            let node = NodeRecord::from_row(row);
            match node {
                Ok(node) => ans.push(node),
                Err(err) => {
                    last_err = Some(err);
                }
            };
        }
        if ans.len() == 0 && limit.unwrap_or(1) > 0 {
            if let Some(err) = last_err {
                error!("Failed to search inodes in database: {:?}", err);
                trace!("-{} -> {:?}", trace_msg, err);
                return Err(err);
            }
        }
        trace!("-{} -> {} nodes", trace_msg, ans.len());
        Ok(ans)
    }

    #[allow(dead_code)]
    pub fn find_one(
        parent: Option<u64>,
        name: Option<&str>,
        obj_type: Option<ObjectType>,
        obj_uuid: Option<Uuid>,
        file_type: Option<FileType>,
        tx: &Transaction,
    ) -> DVResult<NodeRecord> {
        match NodeRecord::search(
            parent,
            name,
            obj_type,
            obj_uuid,
            file_type,
            Some(1),
            None,
            tx,
        ) {
            Ok(v) => {
                if v.len() > 0 {
                    Ok(v[0].clone())
                } else {
                    Err(DVError::SQLError(SQLError::QueryReturnedNoRows))
                }
            }
            Err(err) => Err(err),
        }
    }

    #[allow(dead_code)]
    pub fn get(inode: u64, tx: &Transaction) -> DVResult<NodeRecord> {
        trace!("+NodeRecord::get(inode={})", inode);
        let res = tx.query_row(
        "SELECT `inode`, `nlink`, `obj_uuid`, `obj_type`, `file_type`, `mtime`, `ctime`, `crtime` FROM `node_view_nlink` WHERE `inode` = ?1",
        params![u64_to_i64(inode)],
        |row| Ok(NodeRecord::from_row(row))
    );
        match res {
            Ok(v) => {
                trace!("-{}(inode={}) -> {:?}", stringify!(inode_get), inode, v);
                v
            }
            Err(err) => {
                error!("Failed to get inode {} from database: {:?}", inode, err);
                trace!("-NodeRecord::get(inode={}) -> {:?}", inode, err);
                Err(DVError::SQLError(err))
            }
        }
    }

    #[allow(dead_code)]
    pub fn del(inode: u64, tx: &Transaction) -> SQLResult<()> {
        trace!("+NodeRecord::del(inode={})", inode);
        let res = tx.execute(
            "DELETE FROM `node_meta` WHERE `inode` = ?1",
            params![u64_to_i64(inode)],
        );
        match res {
            Ok(_) => {
                trace!("+NodeRecord::del(inode={}) -> Ok", inode);
                Ok(())
            }
            Err(err) => {
                error!("Failed to delete inode {} from database: {:?}", inode, err);
                trace!("+NodeRecord::del(inode={}) -> {:?}", inode, err);
                Err(err)
            }
        }
    }

    #[allow(dead_code)]
    pub fn to_file_attr(&self, size: u64) -> FileAttr {
        unsafe {
            FileAttr {
                ino: self.inode,
                size: size,
                blocks: size / BLOCK_SIZE,
                atime: self.mtime,
                mtime: self.mtime,
                ctime: self.ctime,
                crtime: self.crtime,
                kind: self.file_type,
                perm: DEFAULT_DIR_PERM,
                nlink: self.nlink,
                uid: DEFAULT_UID,
                gid: DEFAULT_GID,
                rdev: DEFAULT_RDEV,
                flags: DEFAULT_NODE_FLAGS,
                blksize: BLOCK_SIZE as u32,
                padding: DEFAULT_NODE_PADDING,
            }
        }
    }
}

pub fn set_inode_counter(conn: &Connection, inode_next: &AtomicU64) -> Result<(), rusqlite::Error> {
    trace!("+{}", stringify!(set_inode_counter));
    let inode = get_highest_inode(conn);
    if let Err(err) = inode {
        error!("Failed to set INODE_NEXT: {:?}", err);
        trace!("-{} -> {:?}", stringify!(set_inode_counter), err);
        return Err(err);
    }
    let inode = inode.unwrap();
    inode_next.store(inode + 1, Ordering::SeqCst);
    debug!("Set INODE_NEXT to {}", inode);
    trace!("-{}", stringify!(set_inode_counter));
    return Ok(());
}

fn get_highest_inode(conn: &Connection) -> Result<u64, rusqlite::Error> {
    trace!("+{}", stringify!(get_highest_inode));
    // This weird code is because I want u64 but SQLite only stores i64
    let res_max = conn.query_row("SELECT MAX(`inode`) FROM `node_meta`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_max {
        error!("Failed to get MAX(`inode`): {:?}", err);
        return Err(err);
    }
    let res_min = conn.query_row("SELECT MIN(`inode`) FROM `node_meta`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_min {
        error!("Failed to get MIN(`inode`): {:?}", err);
        return Err(err);
    }

    let a = i64_to_u64(res_max.unwrap());
    let b = i64_to_u64(res_min.unwrap());
    if a > b {
        trace!("-{} -> {}", stringify!(get_highest_inode), a);
        Ok(a)
    } else {
        trace!("-{} -> {}", stringify!(get_highest_inode), b);
        Ok(b)
    }
}
