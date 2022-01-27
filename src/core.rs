use crate::prelude::*;
use crate::files::FileNode;
use crate::utils::ensure_dir_exists;
use core::marker::PhantomData;
use crate::schema::open_database;

pub struct DataVirFS<'fs> {
    conn: SQLConnection,
    data_path: PathBuf,
    root_node_uuid: Uuid,
    _phantom_data: PhantomData<&'fs ()>,
}

impl<'fs> DataVirFS<'_> {
    pub(crate) fn new_tx<'tx: 'fs>(&mut self) -> SQLTransaction<'tx> {
        let mut tx = fuck_mut(&mut self.conn).transaction().unwrap();
        tx.set_drop_behavior(rusqlite::DropBehavior::Rollback);
        tx
    }

    pub fn new(data_path: &Path) -> DVResult<Self> {
        let trace_msg = format!(
            "{}(data_path={:?})",
            function!(),
            data_path
        );
        trace!("+{}", trace_msg);
        // Ensure DATA_DIR exists and is a folder
        if let Err(err) = ensure_dir_exists("DATA_DIR", data_path) {
            trace!("-{}", trace_msg);
            return Err(DVError::IOError(err));
        }

        // Open database
        let mut db_path = data_path.to_path_buf();
        db_path.push("datavir.sqlite");
        let conn = match open_database(db_path.as_path()) {
            Ok(v) => v,
            Err(err) => {
                error!("Failed to open database at {:?}: {:?}", db_path, err);
                trace!("-{}", trace_msg);
                return Err(DVError::SQLError(err));
            }
        };
        info!("Database ready!");

        let mut ans = DataVirFS {
            conn: conn,
            root_node_uuid: Uuid::nil(),
            data_path: data_path.to_path_buf(),
            _phantom_data: PhantomData,
        };

        ans.ensure_root_node_uuid()?;

        trace!("-{} -> Ok", trace_msg);
        Ok(ans)
    }

    fn ensure_root_node_uuid(&mut self) -> DVResult<()> {
        let trace_msg = format!("{}()", function!());
        trace!("+{}", trace_msg);

        if !self.root_node_uuid.is_nil() {
            debug!("Already has root_node_uuid = {}", self.root_node_uuid);
            return Ok(())
        }

        let mut tx = self.new_tx();

        let res: rusqlite::Result<Uuid> = tx.query_row(
            "SELECT `value` FROM `app_config` WHERE `key` = 'root_node_uuid'",
            params![],
            |row| row.get(0),
        );
        self.root_node_uuid = match res {
            Ok(v) => v,
            Err(err) => {
                if err == SQLError::QueryReturnedNoRows {
                    Uuid::nil()
                } else {
                    error!("Failed to get root_node_uuid from database: {:?}", err);
                    trace!("-{} -> {:?}", function!(), err);
                    return Err(err)?;
                }
            }
        };
        
        if !self.root_node_uuid.is_nil() {
            debug!("Got root_node_uuid = {}", self.root_node_uuid);
            return Ok(())
        }

        // Create new node
        let mut root_node = FileNode::new_root(Utc::now());
        root_node.save(&mut tx)?;
        debug!("{:?}", root_node);

        // Save root node uuid
        let res = tx.execute(
            "INSERT INTO `app_config` (`key`, `value`) VALUES ('root_node_uuid', ?1)",
            params![root_node.uuid()]
        );
        if let Err(err) = res {
            error!("Failed to set root_node_uuid to {}: {:?}", root_node.uuid(), err);
            trace!("-{} -> {:?}", trace_msg, err);
            return Err(err)?;
        }
        tx.commit()?;

        trace!("-{} -> Ok()", trace_msg);
        Ok(())
    }
}