use crate::open_database;
use crate::prelude::*;
#[allow(unused_imports)]
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;

#[allow(dead_code)]
pub struct DataVirFS {
    conn: Connection,
    data_path: PathBuf,
    mount_path: PathBuf,
    mount_opts: Vec<MountOption>,
}

impl DataVirFS {
    pub fn new(data_path: &Path, mount_path: &Path) -> DVResult<Self> {
        let trace_str = format!(
            "{}(data_path={:?}, mount_path={:?})",
            stringify!(new),
            data_path,
            mount_path
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
        let mut db_path = data_path.to_path_buf();
        db_path.push("datavir.sqlite");
        let conn = match open_database(db_path.as_path()) {
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
        self.ensure_fs_name("datavir");
        self.ensure_auto_unmount();
        let mount_opts_copy = self.mount_opts.clone();
        let mount_path_copy = self.mount_path.clone();
        fuser::mount2(self, mount_path_copy.as_path(), &mount_opts_copy)
    }
}

impl Filesystem for DataVirFS {
    #[allow(unused_variables)]
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        reply.error(ENOENT);
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
