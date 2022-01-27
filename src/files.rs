use crate::prelude::*;

#[derive(Debug)]
pub struct FileNode {
    uuid: Uuid,
    parent: Uuid,
    inode_num: Option<u64>,
    filename: String,
    superhidden: bool,
    contents: Vec<u8>,
    changed_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl FileNode {
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn new_with_uuid(now: DateTime<Utc>, uuid: Uuid, parent: Uuid) -> Self {
        FileNode {
            uuid: uuid,
            parent: parent,
            inode_num: None,
            filename: "".to_string(),
            superhidden: false,
            contents: [].to_vec(),
            changed_at: now,
            created_at: now,
        }
    }

    pub fn new(now: DateTime<Utc>, parent: Uuid) -> Self {
        FileNode::new_with_uuid(now, new_uuid_at(now), parent)
    }

    pub fn new_root(now: DateTime<Utc>) -> Self {
        let uuid = new_uuid_at(now);
        FileNode::new_with_uuid(now, uuid, uuid)
    }

    fn from_row(row: &rusqlite::Row) -> DVResult<FileNode> {
        // SELECT `node_uuid`, `parent_uuid`, `inode_num`, `filename`, `contents`, `super_hidden`, `changed_at`, `created_at` FROM `filenode`
        Ok(FileNode {
            uuid: row.get(0)?,
            parent: row.get(1)?,
            filename: row.get(2)?,
            inode_num: Some(row.get(3)?),
            contents: row.get(4)?,
            superhidden: row.get(5)?,
            changed_at: row.get(6)?,
            created_at: row.get(7)?,
        })
    }

    // meant to be used after saving
    fn load_inode_num(&mut self, tx: &SQLTransaction) -> DVResult<()> {
        let trace_msg = format!("FileNode::load_inode_num(self.uuid={:?})", self.uuid);
        trace!("+{}", trace_msg);

        if self.inode_num.is_some() {
            trace!("-{} -> Ok", trace_msg);
            return Ok(())
        }

        let sql = "SELECT `inode_num` FROM `filenode` WHERE `node_uuid` = ?1";
        let res = tx.query_row(sql, params![self.uuid], |row| {
            Ok(row.get(0)?)
        });
        match res {
            Err(err) => {
                if is_sql_err_not_found(&err) {
                    warn!("Failed to get inode for {}: {:?}", self.uuid, err);
                    trace!("-{} -> Ok", trace_msg);
                    return Ok(());
                } else {
                    error!("Failed to get inode for {}: {:?}", self.uuid, err);
                    trace!("-{} -> Err", trace_msg);
                    return Err(err)?;
                }
            },
            Ok(inode) => {
                self.inode_num = Some(inode);
                trace!("-{} -> Ok", trace_msg);
                return Ok(());
            }
        }
    }

    #[allow(dead_code)]
    pub fn load_by_uuid(uuid: Uuid, tx: &SQLTransaction) -> DVResult<FileNode> {
        let trace_msg = format!("FileNode::load_by_uuid(uuid={:?})", uuid);
        trace!("+{}", trace_msg);

        let sql = "SELECT `node_uuid`, `parent_uuid`, `inode_num`, `filename`, `contents`, `super_hidden`, `changed_at`, `created_at` FROM `filenode` WHERE `node_uuid` = ?1";
        debug!("{}", sql);
        let res = tx.query_row(sql, params![uuid], |row| {
            Ok(FileNode::from_row(row))
        });

        if res.is_err() {
            let err = res.unwrap_err();
            error_or_warn!(
                is_sql_err_not_found(&err), 
                "Failed to get filenode ({}) from database: {:?}",
                uuid, err
            );
            trace!("-{} -> Err", trace_msg);
            return Err(DVError::SQLError(err));
        }

        let mut node = res.unwrap()?;
        node.load_inode_num(tx)?;
        trace!("-{} -> Ok", trace_msg);
        Ok(node)
    }

    #[allow(dead_code)]
    pub fn load_by_inode_num(_inode_num: u64, _tx: &SQLTransaction) -> DVResult<FileNode> {
        todo!()
    }

    pub fn save(&mut self, tx: &SQLTransaction) -> DVResult<()> {
        let res = tx.execute(
            "REPLACE INTO `filenode` (`node_uuid`, `parent_uuid`, `filename`, `contents`, `super_hidden`, `changed_at`, `created_at`) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![self.uuid, self.parent, self.filename, self.contents, self.superhidden, self.changed_at, self.created_at]
        );
        if let Err(err) = res {
            error!("Failed to save filenode uuid={}: {:?}", self.uuid, err);
            trace!("-{} -> {:?}", function!(), err);
            return Err(err)?;
        }

        self.load_inode_num(&tx)?;

        Ok(())
    }
}