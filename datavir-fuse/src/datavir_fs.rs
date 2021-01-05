use crate::inode_record::NodeName;
use crate::inode_record::NodeNameIter;
use crate::inode_record::NodeRecord;
use crate::inode_record::INODE_MIN;
use crate::open_database;
use crate::prelude::*;
use core::sync::atomic::AtomicU64;
use core::time::Duration;
use fuser::ReplyOpen;
use fuser::TimeOrNow;
#[allow(unused_imports)]
use fuser::{FileAttr, FileType, Filesystem, MountOption, Request};
use fuser::{
    ReplyAttr, ReplyBmap, ReplyData, ReplyDirectory, ReplyDirectoryPlus, ReplyEmpty, ReplyEntry,
    ReplyIoctl, ReplyLock, ReplyLseek, ReplyStatfs, ReplyWrite, ReplyXattr,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::time::SystemTime;

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

    fn new_tx<'tx: 'fs>(&mut self) -> Transaction<'tx> {
        let mut tx = fuck_mut(&mut self.conn).transaction().unwrap();
        tx.set_drop_behavior(rusqlite::DropBehavior::Rollback);
        tx
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
    /// Clean up filesystem. Called on filesystem exit.
    fn destroy(&mut self, _req: &Request<'_>) {}

    #[allow(unused_variables)]
    /// Look up a directory entry by name and get its attributes.
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
        let tx = self.new_tx();
        match NodeRecord::lookup(parent, name, &tx) {
            Ok(node) => {
                let attrs = node.to_file_attr();
                trace!("-{} -> Ok", trace_msg);
                reply.entry(&self.basic_ttl, &attrs, 0);
            }
            Err(err) => {
                if err.is_not_found() {
                    warn!("Failed to lookup node ({}, {:?}): {:?}", parent, name, err);
                } else {
                    error!("Failed to lookup node ({}, {:?}): {:?}", parent, name, err);
                }
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
            }
        };
    }

    /// (Not Implemented) Forget about an inode. The nlookup parameter indicates the number of lookups previously performed on this inode. If the filesystem implements inode lifetimes, it is recommended that inodes acquire a single reference on each lookup, and lose nlookup references on each forget. The filesystem may ignore forget calls, if the inodes don't need to have a limited lifetime. On unmount it is not guaranteed, that all referenced inodes will receive a forget message.
    fn forget(&mut self, _req: &Request<'_>, _ino: u64, _nlookup: u64) {
        warn!("FORGET - Not Implemented");
    }

    #[allow(unused_variables)]
    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let trace_msg = format!(
            "DataVirFS::getattr(_req={}, ino={})",
            fmt_request(_req),
            ino
        );
        trace!("+{}", trace_msg);

        // get inode and repy the attributes
        let node = match NodeRecord::get(ino, &self.new_tx()) {
            Ok(v) => v,
            Err(err) => {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
                return;
            }
        };
        let attrs = node.to_file_attr();
        reply.attr(&self.basic_ttl, &attrs);
        trace!("-{} -> {:?}", trace_msg, attrs);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Set file attributes.
    fn setattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        warn!("SETATTR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Read symbolic link.
    fn readlink(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyData) {
        warn!("READLINK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Create file node. Create a regular file, character device, block device, fifo or socket node.
    fn mknod(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        warn!("MKNOD - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Create a directory.
    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        warn!("MKDIR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Remove a file.
    fn unlink(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        warn!("UNLINK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Remove a directory.
    fn rmdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        warn!("RMDIR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Create a symbolic link.
    fn symlink(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _link: &Path,
        reply: ReplyEntry,
    ) {
        warn!("SYMLINK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Rename a file.
    fn rename(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _newparent: u64,
        _newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        warn!("RENAME - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Create a hard link.
    fn link(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        warn!("LINK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Open a file. Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are available in flags. Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and use this in other all other file operations (read, write, flush, release, fsync). Filesystem may also implement stateless file I/O and not store anything in fh. There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details.
    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        warn!("OPEN - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Read data. Read should send exactly the number of bytes requested except on EOF or error, otherwise the rest of the data will be substituted with zeroes. An exception to this is when the file has been opened in 'direct_io' mode, in which case the return value of the read system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn't set any value.
    ///
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
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
        warn!("READ - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Write data. Write should return exactly the number of bytes requested except on error. An exception to this is when the file has been opened in 'direct_io' mode, in which case the return value of the write system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn't set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set, the pid, uid, gid, and fh may not match the value that would have been sent if write cachin is disabled flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        warn!("WRITE - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Flush method. This is called on each close() of the opened file. Since file descriptors can be duplicated (dup, dup2, fork), for one open call there may be many flush calls. Filesystems shouldn't assume that flush will always be called after some writes, or that if will be called at all. fh will contain the value set by the open method, or will be undefined if the open method didn't set any value. NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not forced to flush pending writes. One reason to flush data, is if the filesystem wants to return write errors. If the filesystem supports file locking operations (setlk, getlk) it should remove all locks belonging to 'lock_owner'.
    fn flush(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        warn!("WRITE - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Release an open file. Release is called when there are no more references to an open file: all file descriptors are closed and all memory mappings are unmapped. For every open call there will be exactly one release call. The filesystem may reply with an error, but error values are not returned to close() or munmap() which triggered the release. fh will contain the value set by the open method, or will be undefined if the open method didn't set any value. flags will contain the same flags as for open.
    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        warn!("RELEASE - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Synchronize file contents. If the datasync parameter is non-zero, then only the user data should be flushed, not the meta data.
    fn fsync(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        warn!("FSYNC - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    /// Open a directory. Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and use this in other all other directory stream operations (readdir, releasedir, fsyncdir). Filesystem may also implement stateless directory I/O and not store anything in fh, though that makes it impossible to implement standard conforming directory stream operations in case the contents of the directory can change between opendir and releasedir.
    fn opendir(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        // TODO: see what to do with the flags
        let trace_msg = format!(
            "DataVirFS::opendir(_req={}, ino={}, flags={})",
            fmt_request(_req),
            ino,
            flags
        );
        trace!("+{}", trace_msg);

        // get inode and check if it is a directory
        let tx = self.new_tx();
        let node = match NodeRecord::get(ino, &tx) {
            Ok(v) => v,
            Err(err) => {
                trace!("-{} -> {:?}", trace_msg, err);
                reply.error(i32::from(err));
                return;
            }
        };
        if !node.can_readdir(&tx) {
            trace!("-{} -> Not dir", trace_msg);
            reply.error(POSIX_NOT_A_DIRECTORY);
            return;
        }

        // TODO: use fh to store the transaction object and the iterator (this will proabbly increase performance and consistency)
        let fh = 0;
        let reply_flags = 0;
        reply.opened(fh, reply_flags);
        trace!("-{} -> Ok", trace_msg);
    }

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    /// Read directory. Send a buffer filled using buffer.fill(), with size not exceeding the requested size. Send an empty buffer on end of stream. fh will contain the value set by the opendir method, or will be undefined if the opendir method didn't set any value.
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
            let full = reply.add(ino, offset + 1, FileType::Directory, ".");
            debug!("Add '.' for {}", ino);
            if full {
                reply.ok();
                trace!("-{}", trace_msg);
                return;
            } else {
                offset += 1;
            }
        }
        let tx = self.new_tx();
        if offset == 1 {
            let parent = match NodeRecord::get_main_parent_for(ino, &tx) {
                Ok(v) => v,
                Err(err) => {
                    trace!("-{} -> {:?}", trace_msg, err);
                    reply.error(i32::from(err));
                    return;
                }
            };
            let full = reply.add(parent, offset + 1, FileType::Directory, "..");
            debug!("Add '..' for {}", ino);
            if full {
                reply.ok();
                trace!("-{}", trace_msg);
                return;
            } else {
                offset += 1;
            }
        }

        let list = NodeName::list(ino, false, Some(offset - 2), &tx);

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
                offset + 1,
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
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Read directory. Send a buffer filled using buffer.fill(), with size not exceeding the requested size. Send an empty buffer on end of stream. fh will contain the value set by the opendir method, or will be undefined if the opendir method didn't set any value.
    fn readdirplus(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        warn!("READDIRPLUS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Release an open directory. For every opendir call there will be exactly one releasedir call. fh will contain the value set by the opendir method, or will be undefined if the opendir method didn't set any value.
    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        warn!("RELEASEDIR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Synchronize directory contents. If the datasync parameter is set, then only the directory contents should be flushed, not the meta data. fh will contain the value set by the opendir method, or will be undefined if the opendir method didn't set any value.
    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        warn!("FSYNCDIR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Get file system statistics.
    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        warn!("STATFS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Set an extended attribute.
    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _value: &[u8],
        _flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        warn!("SETXATTRS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Get an extended attribute. If size is 0, the size of the value should be sent with reply.size(). If size is not 0, and the value fits, send it with reply.data(), or reply.error(ERANGE) if it doesn't.
    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        warn!("GETXATTRS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) List extended attribute names. If size is 0, the size of the value should be sent with reply.size(). If size is not 0, and the value fits, send it with reply.data(), or reply.error(ERANGE) if it doesn't.
    fn listxattr(&mut self, _req: &Request<'_>, _ino: u64, _size: u32, reply: ReplyXattr) {
        warn!("LISTXATTRS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Remove an extended attribute.
    fn removexattr(&mut self, _req: &Request<'_>, _ino: u64, _name: &OsStr, reply: ReplyEmpty) {
        warn!("REMOVEXATTR - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Check file access permissions. This will be called for the access() system call. If the 'default_permissions' mount option is given, this method is not called. This method is not called under Linux kernel versions 2.4.x
    fn access(&mut self, _req: &Request<'_>, _ino: u64, _mask: i32, reply: ReplyEmpty) {
        warn!("ACCESS - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    // Create is intentionally not implemented

    #[allow(unused_variables)]
    /// Create and open a file. If the file does not exist, first create it with the specified mode, and then open it. Open flags (with the exception of O_NOCTTY) are available in flags. Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and use this in other all other file operations (read, write, flush, release, fsync). There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details. If this method is not implemented or under Linux kernel versions earlier than 2.6.15, the mknod() and open() methods will be called instead.
    fn getlk(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        reply: ReplyLock,
    ) {
        warn!("GETLK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// Acquire, modify or release a POSIX file lock. For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but otherwise this is not always the case. For checking lock ownership, 'fi->owner' must be used. The l_pid field in 'struct flock' should only be used to fill in this field in getlk(). Note: if the locking methods are not implemented, the kernel will still allow file locking to work locally. Hence these are only interesting for network filesystems and similar.
    fn setlk(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: i32,
        _pid: u32,
        _sleep: bool,
        reply: ReplyEmpty,
    ) {
        warn!("SETLK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Map block index within file to block index within device. Note: This makes sense only for block device backed filesystems mounted with the 'blkdev' option
    fn bmap(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _blocksize: u32,
        _idx: u64,
        reply: ReplyBmap,
    ) {
        warn!("BMAP - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Control and comunicate with the FS
    fn ioctl(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _cmd: u32,
        _in_data: &[u8],
        _out_size: u32,
        reply: ReplyIoctl,
    ) {
        // I might use it for "communication"
        warn!("IOCTL - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Rreallocate or deallocate space to a file
    fn fallocate(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _length: i64,
        _mode: i32,
        reply: ReplyEmpty,
    ) {
        warn!("FALLOCATE - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Reposition read/write file offset
    fn lseek(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _whence: i32,
        reply: ReplyLseek,
    ) {
        warn!("LSEEK - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
    }

    #[allow(unused_variables)]
    /// (Not Implemented) Copy the specified range from the source inode to the destination inode
    fn copy_file_range(
        &mut self,
        _req: &Request<'_>,
        _ino_in: u64,
        _fh_in: u64,
        _offset_in: i64,
        _ino_out: u64,
        _fh_out: u64,
        _offset_out: i64,
        _len: u64,
        _flags: u32,
        reply: ReplyWrite,
    ) {
        warn!("COPY_FILE_RANGE - Not Implemented");
        reply.error(POSIX_NOT_IMPLEMENTED);
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
