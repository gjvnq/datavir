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

#[allow(dead_code)]
const INODE_ROOT: u64 = 1;
#[allow(dead_code)]
const INODE_CONFIG: u64 = 2;
#[allow(dead_code)]
const INODE_SOCKET: u64 = 3;
#[allow(dead_code)]
const INODE_BUNDLES_TXT: u64 = 4;
#[allow(dead_code)]
const INODE_FILTERS_DIR: u64 = 5;
#[allow(dead_code)]
const INODE_ALL_BUNDLES_DIR: u64 = 6;
#[allow(dead_code)]
const INODE_BUNDLES_DIR: u64 = 7;

pub const INODE_MIN: i64 = 64;
static INODE_NEXT: AtomicU64 = AtomicU64::new(INODE_MIN as u64);

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
const DEFAULT_RDEV: u32 = 0; //no idea
const DEFAULT_NODE_FLAGS: u32 = 0;
const DEFAULT_NODE_PADDING: u32 = 0;

pub trait INodeRegisterable: std::fmt::Debug {
    fn get_uuid(&self) -> Uuid;
    fn get_obj_type(&self) -> ObjectType;
    fn get_path(&self) -> String;
}

#[inline]
fn i64_to_u64(num: i64) -> u64 {
    unsafe { std::mem::transmute(num) }
}

#[inline]
fn u64_to_i64(num: u64) -> i64 {
    unsafe { std::mem::transmute(num) }
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

fn string2file_type(str: &str) -> DVResult<FileType> {
    unimplemented!()
}

fn file_type2string(kind: FileType) -> String {
    unimplemented!()
}

#[derive(Debug)]
struct INodeRecord {
    inode_num: u64,
    obj_uuid: Uuid,
    obj_type: ObjectType,
    file_type: FileType,
    path: String,
    mtime: SystemTime,
    ctime: SystemTime,
    crtime: SystemTime,
}

impl INodeRecord {
    fn new(
        num: u64,
        uuid: Uuid,
        obj_type: ObjectType,
        file_type: FileType,
        path: String,
        mtime: SystemTime,
        ctime: SystemTime,
        crtime: SystemTime,
    ) -> Self {
        INodeRecord {
            inode_num: num,
            obj_uuid: uuid,
            obj_type: obj_type,
            file_type: file_type,
            path: path,
            mtime: mtime,
            ctime: ctime,
            crtime: crtime,
        }
    }

    fn new_sql(
        num: i64,
        uuid: Uuid,
        obj_type: ObjectType,
        file_type: String,
        path: String,
        mtime: i64,
        ctime: i64,
        crtime: i64,
    ) -> DVResult<Self> {
        let num = i64_to_u64(num);
        let mtime = sql2time(mtime)?;
        let ctime = sql2time(ctime)?;
        let crtime = sql2time(crtime)?;
        let file_type = string2file_type(&file_type)?;

        Ok(INodeRecord::new(
            num, uuid, obj_type, file_type, path, mtime, ctime, crtime,
        ))
    }

    #[allow(dead_code)]
    pub fn new_now(
        num: u64,
        uuid: Uuid,
        obj_type: ObjectType,
        file_type: FileType,
        path: String,
    ) -> Self {
        let now = SystemTime::now();
        INodeRecord::new(num, uuid, obj_type, file_type, path, now, now, now)
    }

    #[allow(dead_code)]
    pub fn new_now_next(
        uuid: Uuid,
        obj_type: ObjectType,
        file_type: FileType,
        path: String,
    ) -> Self {
        let now = SystemTime::now();
        let num = get_next_inode();
        INodeRecord::new(num, uuid, obj_type, file_type, path, now, now, now)
    }

    fn get_inode_i64(&self) -> i64 {
        u64_to_i64(self.inode_num)
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, _tx: &Transaction) -> DVResult<()> {
        unimplemented!()
    }

    #[allow(dead_code)]
    // if new_path is None, it won't be changed
    pub fn update_metadata(&mut self, new_path: Option<&str>, tx: &Transaction) -> DVResult<()> {
        trace!("+INodeRecord::update_metadata(self={:?})", self);
        let now = SystemTime::now();
        let now_unix = time2sql(now)?;
        let sql = match new_path {
            Some(_) => "UPDATE `inode` SET `ctime` = ?1, `path` = ?2 WHERE `inode_num` = ?3",
            None => "UPDATE `inode` SET `ctime` = ?1, WHERE `inode_num` = ?3",
        };
        let res = tx.execute(sql, params![now_unix, new_path, self.get_inode_i64()]);
        if let Err(err) = res {
            error!("Failed to update metadata for INodeRecord: {:?}", err);
            trace!(
                "-INodeRecord::update_metadata(self={:?}) -> {:?}",
                self,
                err
            );
            Err(DVError::SQLError(err))
        } else {
            self.ctime = now;
            if let Some(new_path) = new_path {
                self.path = new_path.to_string();
            }
            debug!("Updated metadata for {:?}", self);
            trace!("-INodeRecord::update_metadata(self={:?}) -> Ok", self);
            Ok(())
        }
    }

    #[allow(dead_code)]
    // should be called when the file contents are changed
    pub fn update_mtime(&mut self, tx: &Transaction) -> DVResult<()> {
        trace!("+INodeRecord::update_mtime(self={:?})", self);
        let now = SystemTime::now();
        let now_unix = time2sql(now)?;
        let res = tx.execute(
            "INSERT OR REPLACE INTO `inode` (`inode_num`, `mtime`) (?1, ?2)",
            params![self.get_inode_i64(), now_unix],
        );
        if let Err(err) = res {
            error!("Failed to update metadata for INodeRecord: {:?}", err);
            trace!(
                "-{}(self={:?}) -> {:?}",
                stringify!(INodeRecord::update_mtime),
                self,
                err
            );
            Err(DVError::SQLError(err))
        } else {
            self.mtime = now;
            debug!("Updated mtime for {:?}", self);
            trace!("-INodeRecord::update_mtime(self={:?}) -> Ok", self);
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn get(inode_num: u64, tx: &Transaction) -> DVResult<INodeRecord> {
        trace!("+INodeRecord::get(inode_num={})", inode_num);
        let res = tx.query_row(
        "SELECT `inode_num`, `object_uuid`, `object_type`, `file_type` `mtime`, `ctime`, `crtime`, `path` FROM `inode` WHERE `inode_num` = ?1",
        params![u64_to_i64(inode_num)],
        |row| Ok(INodeRecord::new_sql(row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
    );
        match res {
            Ok(v) => {
                trace!(
                    "-{}(inode_num={}) -> {:?}",
                    stringify!(inode_get),
                    inode_num,
                    v
                );
                v
            }
            Err(err) => {
                error!("Failed to get inode {} from database: {:?}", inode_num, err);
                trace!("-INodeRecord::get(inode_num={}) -> {:?}", inode_num, err);
                Err(DVError::SQLError(err))
            }
        }
    }

    #[allow(dead_code)]
    pub fn del(inode_num: u64, tx: &Transaction) -> SQLResult<()> {
        trace!("+INodeRecord::del(inode_num={})", inode_num);
        let res = tx.execute(
            "DELETE FROM `inode` WHERE `inode_num` = ?1",
            params![u64_to_i64(inode_num)],
        );
        match res {
            Ok(_) => {
                trace!("+INodeRecord::del(inode_num={}) -> Ok", inode_num);
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to delete inode {} from database: {:?}",
                    inode_num, err
                );
                trace!("+INodeRecord::del(inode_num={}) -> {:?}", inode_num, err);
                Err(err)
            }
        }
    }

    #[allow(dead_code)]
    pub fn to_file_attr(&self, size: u64) -> FileAttr {
        unsafe {
            FileAttr {
                ino: INODE_ROOT,
                size: size,
                blocks: size / BLOCK_SIZE,
                atime: self.mtime,
                mtime: self.mtime,
                ctime: self.ctime,
                crtime: self.crtime,
                kind: self.file_type,
                perm: DEFAULT_DIR_PERM,
                nlink: MIN_HARD_LINKS_DIR,
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

pub fn set_inode_counter(conn: &Connection) -> Result<(), rusqlite::Error> {
    trace!("+{}", stringify!(set_inode_counter));
    let inode_num = get_highest_inode(conn);
    if let Err(err) = inode_num {
        error!("Failed to set INODE_NEXT: {:?}", err);
        trace!("-{} -> {:?}", stringify!(set_inode_counter), err);
        return Err(err);
    }
    let inode_num = inode_num.unwrap();
    INODE_NEXT.store(inode_num + 1, Ordering::SeqCst);
    debug!("Set INODE_NEXT to {}", inode_num);
    trace!("-{}", stringify!(set_inode_counter));
    return Ok(());
}

fn get_highest_inode(conn: &Connection) -> Result<u64, rusqlite::Error> {
    trace!("+{}", stringify!(get_highest_inode));
    // This weird code is because I want u64 but SQLite only stores i64
    let res_max = conn.query_row("SELECT MAX(`inode_num`) FROM `inode`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_max {
        error!("Failed to get MAX(`inode_num`): {:?}", err);
        return Err(err);
    }
    let res_min = conn.query_row("SELECT MIN(`inode_num`) FROM `inode`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_min {
        error!("Failed to get MIN(`inode_num`): {:?}", err);
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

fn get_next_inode() -> u64 {
    INODE_NEXT.fetch_add(1, Ordering::SeqCst)
}
