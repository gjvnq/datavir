use crate::inode_record::INodeRecord;
use crate::inode_record::INODE_MIN;
use crate::inode_record::INODE_ROOT;
use crate::open_database;
use crate::prelude::*;
use core::sync::atomic::AtomicU64;
use core::time::Duration;
#[allow(unused_imports)]
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;

#[allow(dead_code)]
#[derive(Debug)]
pub struct DataVirFS {
    conn: Connection,
    data_path: PathBuf,
    mount_path: PathBuf,
    mount_opts: Vec<MountOption>,
    inode_next: AtomicU64,
    basic_ttl: Duration,
}

impl DataVirFS {
    pub fn new(data_path: &Path, mount_path: &Path) -> DVResult<Self> {
        let trace_str = format!(
            "DataVirFS::new(data_path={:?}, mount_path={:?})",
            data_path, mount_path
        );
        trace!("+{}", trace_str);
        // Ensure DATA_DIR exists and is a folder
        if let Err(err) = ensure_dir_exists("DATA_DIR", data_path) {
            trace!("-{}", trace_str);
            return Err(DVError::IOError(err));
        }

        // Ensure MOUNT_POINT exists and is a folder
        if let Err(err) = ensure_dir_exists("MOUNT_POINT", mount_path) {
            trace!("-{}", trace_str);
            return Err(DVError::IOError(err));
        }

        // FUSE doesn't like when the mount point is not empty
        let mount_path_listing = mount_path.read_dir();
        match mount_path_listing {
            Ok(mut dir_listing) => {
                let is_empty = dir_listing.next().is_none();
                if !is_empty {
                    error!("Mount point {:?} is not is_empty", mount_path);
                    trace!("-{}", trace_str);
                    return Err(DVError::DirNotClean(mount_path.to_path_buf()));
                }
                // dir is clean, we are good to go
            }
            Err(err) => {
                error!("Failed to get contents of {:?}", mount_path);
                trace!("-{}", trace_str);
                return Err(DVError::IOError(err));
            }
        }

        // Open database
        let inode_next = AtomicU64::new(INODE_MIN as u64);
        let mut db_path = data_path.to_path_buf();
        db_path.push("datavir.sqlite");
        let conn = match open_database(db_path.as_path(), &inode_next) {
            Ok(v) => v,
            Err(err) => {
                error!("Failed to open database at {:?}: {:?}", db_path, err);
                trace!("-{}", trace_str);
                return Err(DVError::SQLError(err));
            }
        };
        info!("Database ready!");

        trace!("-{} -> Ok", trace_str);
        Ok(DataVirFS {
            conn: conn,
            data_path: data_path.to_path_buf(),
            mount_path: mount_path.to_path_buf(),
            mount_opts: vec![],
            inode_next: inode_next,
            basic_ttl: Duration::from_secs(1),
        })
    }

    fn ensure_fs_name(&mut self, default_name: &str) {
        for opt in &self.mount_opts {
            if let MountOption::FSName(_) = opt {
                return;
            }
        }
        self.mount_opts
            .push(MountOption::FSName(default_name.to_string()));
    }

    // It doesn't seem to be working
    fn ensure_auto_unmount(&mut self) {
        for opt in &self.mount_opts {
            if let MountOption::AutoUnmount = opt {
                return;
            }
        }
        self.mount_opts.push(MountOption::AutoUnmount);
    }

    pub fn mount(mut self) -> IOResult<()> {
        let self_str = format!("{:?}", self);
        trace!("+DataVirFS::mount(self={})", self_str);
        self.ensure_fs_name("datavir");
        self.ensure_auto_unmount();
        let mount_opts_copy = self.mount_opts.clone();
        let mount_path_copy = self.mount_path.clone();
        let ans = fuser::mount2(self, mount_path_copy.as_path(), &mount_opts_copy);
        trace!("-DataVirFS::mount(self={})", self_str);
        ans
    }

    fn new_tx(&mut self) -> Transaction {
        self.conn.transaction().unwrap()
    }

    fn find_in_root(&mut self, parent: u64, name: &str) -> DVResult<FileAttr> {
        let mut name = name;
        if parent == 1 {
            name = match name {
                ".Trash" => "Trash",
                name => name,
            };
        }
        trace!("{:?}", name);
        let ans = INodeRecord::find_one(
            Some(parent),
            Some(name),
            Some(ObjectType::Reserved),
            None,
            None,
            &self.new_tx(),
        );
        debug!("{:?}", ans);
        match ans {
            Ok(v) => Ok(v.to_file_attr(0)),
            Err(err) => Err(err),
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &str) -> DVResult<FileAttr> {
        if parent == 1 {
            return self.find_in_root(parent, name);
        }
        Err(DVError::NotImplemented)
    }
}

impl panic::UnwindSafe for DataVirFS {}

impl panic::RefUnwindSafe for DataVirFS {}

fn fmt_request(req: &Request) -> String {
    format!(
        "Request{{unique: {}, uid: {}, gid: {}, pid: {}}}",
        req.unique(),
        req.uid(),
        req.gid(),
        req.pid()
    )
}

impl Filesystem for DataVirFS {
    #[allow(unused_variables)]
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let trace_msg = format!(
            "DataVirFS::lookup(_req={}, parent={}, name={:?})",
            fmt_request(_req),
            parent,
            name
        );
        trace!("+{}", trace_msg);
        let name = match name.to_str() {
            Some(s) => s,
            None => {
                error!("Failed to convert OsStr: {:?}", name);
                reply.error(POSIX_IO_ERROR);
                return;
            }
        };
        match DataVirFS::lookup(self, _req, parent, name) {
            Ok(attr) => reply.entry(&self.basic_ttl, &attr, 0),
            Err(err) => {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
            }
        }
    }

    #[allow(unused_variables)]
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        reply.error(ENOENT);
    }

    #[allow(unused_variables)]
    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        reply.error(ENOENT);
    }

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let trace_msg = format!(
            "DataVirFS::readdir(_req={}, ino={}, offset={})",
            fmt_request(_req),
            ino,
            offset
        );
        trace!("+{}", trace_msg);

        if ino == INODE_ROOT {
            if offset == 0 {
                if reply.add(INODE_ROOT, 1, FileType::Directory, ".") {
                    reply.ok();
                    return;
                }
                if reply.add(INODE_ROOT, 2, FileType::Directory, "..") {
                    reply.ok();
                    return;
                }
            }

            // The -2 is because of the . and ..
            let list = INodeRecord::search(
                Some(ino),
                None,
                Some(ObjectType::Reserved),
                None,
                None,
                Some(32),
                Some(offset - 2),
                &self.new_tx(),
            );
            if let Err(err) = list {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
                return;
            }
            let mut counter = offset;
            for node in list.unwrap() {
                counter += 1;
                // The unwrap is safe because the nodes came right form the database
                if reply.add(
                    node.get_inode_num().unwrap(),
                    counter,
                    node.get_file_type(),
                    node.get_name(),
                ) {
                    break;
                }
                debug!(
                    "{} {} {:?} {:?}",
                    node.get_inode_num().unwrap(),
                    0,
                    node.get_file_type(),
                    node.get_name()
                );
            }
            reply.ok();
            info!("Replied ROOT");
            return;
        }

        // get inode and check if it is a directory
        let node = match INodeRecord::get(ino, &self.new_tx()) {
            Ok(v) => v,
            Err(err) => {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
                return;
            }
        };
        if node.get_file_type() != FileType::Directory {
            reply.error(POSIX_NOT_A_DIRECTORY);
            return;
        }

        // match DataVirFS::readdir(self, _req, ino, _fh, offset) {
        //     Ok(attr) => reply.entry(&self.basic_ttl, &attr, 0),
        //     Err(err) => {
        //         trace!("-{} -> {:?}", trace_msg, err);
        //         reply.error(i32::from(err));
        //     }
        // }
        reply.error(ENOENT);
    }
}

fn ensure_dir_exists(what: &str, path: &Path) -> IOResult<()> {
    trace!(
        "+{}:(what={:?}, path={:?})",
        stringify!(ensure_dir_exists),
        what,
        path
    );
    if !path.exists() {
        debug!(
            "{} does not exist, will try to make it if parent exists",
            what
        );
        let parent = path.parent();
        debug!("{}.parent() = {:?}", what, parent);
        if let Some(parent) = parent {
            if parent.exists() {
                match fs::create_dir(path) {
                    Ok(_) => {
                        debug!("Created {}", what);
                        trace!(
                            "-{}:(what={:?}, path={:?}) -> {:?}",
                            stringify!(ensure_dir_exists),
                            what,
                            path,
                            true
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        error!("Failed to make {}: {:?}", what, err);
                        trace!(
                            "-{}:(what={:?}, path={:?}) -> {:?}",
                            stringify!(ensure_dir_exists),
                            what,
                            path,
                            false
                        );
                        return Err(err);
                    }
                }
            } else {
                debug!("{}'s parent does not exist", what);
                error!("{} {:?} does not exists", what, path);
                trace!(
                    "-{}:(what={:?}, path={:?}) -> {:?}",
                    stringify!(ensure_dir_exists),
                    what,
                    path,
                    false
                );
                return Err(IOError::new(IOErrorKind::NotFound, ""));
            }
        } else {
            error!("Failed to get {}'s parent", what);
            error!("{} {:?} does not exists", what, path);
            trace!(
                "-{}:(what={:?}, path={:?}) -> {:?}",
                stringify!(ensure_dir_exists),
                what,
                path,
                false
            );
            return Err(IOError::new(
                IOErrorKind::NotFound,
                format!("{:?} is not a directory", path),
            ));
        }
    } else if !path.is_dir() {
        error!("{:?} is not a directory", path);
        trace!(
            "-{}:(what={:?}, path={:?}) -> {:?}",
            stringify!(ensure_dir_exists),
            what,
            path,
            false
        );
        return Err(IOError::new(
            IOErrorKind::NotFound,
            format!("{:?} is not a directory", path),
        ));
    } else {
        debug!("Great! {} exists and is a folder", what);
        trace!(
            "-{}:(what={:?}, path={:?}) -> {:?}",
            stringify!(ensure_dir_exists),
            what,
            path,
            true
        );
        return Ok(());
    }
}
