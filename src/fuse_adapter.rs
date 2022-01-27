use crate::prelude::*;
use crate::schema::open_database;
use std::marker::PhantomData;
#[allow(unused_imports)]
use fuser::{FileAttr, FileType, Filesystem, MountOption, Request};
#[allow(unused_imports)]
use fuser::{
    ReplyAttr, ReplyBmap, ReplyOpen, ReplyData, ReplyDirectory, ReplyDirectoryPlus, ReplyEmpty, ReplyEntry,
    ReplyIoctl, ReplyLock, ReplyLseek, ReplyStatfs, ReplyWrite, ReplyXattr, TimeOrNow
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct DataVirFuseFS<'fs> {
    conn: SQLConnection,
    data_path: PathBuf,
    mount_path: PathBuf,
    mount_opts: Vec<MountOption>,
    // basic_ttl: Duration,
    // fh_name_iter: Mutex<HashMap<u64, NodeNameIter<'fs>>>,
    _phantom_data: PhantomData<&'fs ()>,
}


impl<'fs> DataVirFuseFS<'fs> {
    pub fn new(data_path: &Path, mount_path: &Path) -> DVResult<Self> {
        let trace_str = format!(
            "DataVirFuseFS::new(data_path={:?}, mount_path={:?})",
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
                    return Err(DVError::DirNotClear(mount_path.to_path_buf()));
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
        Ok(DataVirFuseFS {
            conn: conn,
            data_path: data_path.to_path_buf(),
            mount_path: mount_path.to_path_buf(),
            mount_opts: vec![],
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
        trace!("+DataVirFuseFS::mount(self={})", self_str);
        self.ensure_fs_name("datavir");
        self.ensure_auto_unmount();
        let mount_opts_copy = self.mount_opts.clone();
        let mount_path_copy = self.mount_path.clone();
        let ans = fuser::mount2(self, mount_path_copy.as_path(), &mount_opts_copy);
        trace!("-DataVirFuseFS::mount(self={})", self_str);
        ans
    }

    fn new_tx<'tx: 'fs>(&mut self) -> SQLTransaction<'tx> {
        let mut tx = fuck_mut(&mut self.conn).transaction().unwrap();
        tx.set_drop_behavior(rusqlite::DropBehavior::Rollback);
        tx
    }
}

impl<'fs> Filesystem for DataVirFuseFS<'fs> {
}
