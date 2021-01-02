use fuser::ReplyOpen;
use std::collections::HashMap;
use crate::inode_record::NodeNameIter;
use crate::inode_record::NodeName;
use crate::inode_record::NodeRecord;
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
use std::marker::PhantomData;

#[allow(dead_code)]
#[derive(Debug)]
pub struct DataVirFS<'fs> {
    conn: Connection,
    /// This mutex is intended only for last_insert_rowid
    db_mutex: Mutex<()>,
    data_path: PathBuf,
    mount_path: PathBuf,
    mount_opts: Vec<MountOption>,
    inode_next: AtomicU64,
    basic_ttl: Duration,
    fh_name_iter: Mutex<HashMap<u64, NodeNameIter<'fs>>>,
    _phantom_data: PhantomData<&'fs ()>,
}

impl<'fs> DataVirFS<'fs> {
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
            db_mutex: Mutex::new(()),
            data_path: data_path.to_path_buf(),
            mount_path: mount_path.to_path_buf(),
            mount_opts: vec![],
            inode_next: inode_next,
            basic_ttl: Duration::from_secs(1),
            fh_name_iter: Mutex::new(HashMap::new()),
            _phantom_data: PhantomData,
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
}

fn fmt_request(req: &Request) -> String {
    format!(
        "Request{{unique: {}, uid: {}, gid: {}, pid: {}}}",
        req.unique(),
        req.uid(),
        req.gid(),
        req.pid()
    )
}

impl<'fs> Filesystem for DataVirFS<'fs> {
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
        reply.error(POSIX_IO_ERROR);
        // match DataVirFS::lookup(self, _req, parent, name) {
        //     Ok(attr) => reply.entry(&self.basic_ttl, &attr, 0),
        //     Err(err) => {
        //         trace!("-{} -> {:?}", trace_msg, err);
        //         reply.error(i32::from(err));
        //     }
        // }
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
    fn opendir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        flags: i32,
        reply: ReplyOpen) {
        // TODO: see what to do with the flags
        let trace_msg = format!(
            "DataVirFS::opendir(_req={}, ino={}, flags={})",
            fmt_request(_req),
            ino,
            flags
        );
        trace!("+{}", trace_msg);

        // get inode and check if it is a directory
        let node = match NodeRecord::get(ino, &self.new_tx()) {
            Ok(v) => v,
            Err(err) => {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
                return;
            }
        };
        if !node.can_readdir(&self.conn) {
            trace!("-{} -> Not dir", trace_msg);
            reply.error(POSIX_NOT_A_DIRECTORY);
            return;
        }
        let fh = 0;
        let reply_flags = 0;
        reply.opened(fh, reply_flags);

        trace!("-{}", trace_msg);
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
        let mut offset = offset;

        if offset == 0 {
            let full = reply.add(ino, offset, FileType::Directory, ".");
            if full {
                reply.ok();
                trace!("-{}", trace_msg);
                return;
            } else {
                offset += 1;
            }
        }
        if offset == 1 {
            let parent = ino; //TODO: fix this
            let full = reply.add(parent, offset, FileType::Directory, "..");
            if full {
                reply.ok();
                trace!("-{}", trace_msg);
                return;
            } else {
                offset += 1;
            }
        }

        let list = NodeName::find(ino, false, Some(offset-2), &self.conn);
        
        if let Err(err) = list {
            trace!("-{} -> {:?}", trace_msg, err);
            reply.error(i32::from(err));
            return;
        }
        for node in list.unwrap() {
            offset += 1;
            // The unwrap is safe because the nodes came right form the database
            if reply.add(
                node.get_inode(),
                offset,
                node.get_file_type().unwrap(),
                node.get_name(),
            ) {
                break;
            }
            debug!("{:?}", node);
        }
        reply.ok();
        trace!("-{} -> OK", trace_msg);
        info!("Replied dir");
        // return;

        // // get inode and check if it is a directory
        // let node = match NodeRecord::get(ino, &self.new_tx()) {
        //     Ok(v) => v,
        //     Err(err) => {
        //         trace!("-{} -> {:?}", trace_msg, err);
        //         reply.error(i32::from(err));
        //         return;
        //     }
        // };
        // if node.get_file_type() != FileType::Directory {
        //     reply.error(POSIX_NOT_A_DIRECTORY);
        //     return;
        // }

        // match DataVirFS::readdir(self, _req, ino, _fh, offset) {
        //     Ok(attr) => reply.entry(&self.basic_ttl, &attr, 0),
        //     Err(err) => {
        //         trace!("-{} -> {:?}", trace_msg, err);
        //         reply.error(i32::from(err));
        //     }
        // }
        // reply.error(ENOENT);
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
