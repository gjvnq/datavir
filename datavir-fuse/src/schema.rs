use crate::prelude::*;

#[derive(Debug)]
struct SchemaItem<'a> {
    table: (&'a str, &'a str),
    indexes: Vec<(&'a str, &'a str)>,
}

fn schema_upgrade_to_v1(conn: &Connection) -> SQLResult<()> {
    trace!("+{}", stringify!(schema_upgrade_to_v1));
    let mut v1_schema: Vec<SchemaItem> = vec![];
    v1_schema.push(SchemaItem {
        table: (
            "bundles",
            "CREATE TABLE IF NOT EXISTS `bundles` (\
            `bundle_uuid` CHAR(36),\
            `conflicts_from` CHAR(36),\
            `sync_status` INT,\
            `modified` TIMESTAMP,\
            `name` VARCHAR(250));",
        ),
        indexes: vec![(
            "bundles_sync_status_idx",
            "CREATE INDEX IF NOT EXISTS `bundles_sync_status_idx` ON `bundles` (`sync_status`);",
        )],
    });
    v1_schema.push(SchemaItem {
        table: (
            "files",
            "CREATE TABLE IF NOT EXISTS `files` (\
            `file_uuid` CHAR(36),\
            `bundle_uuid` CHAR(36),\
            `modified` TIMESTAMP,\
            `base_blob_uuid` CHAR(36),\
            `tree_hash` VARCHAR(200),\
            `kind` CHAR(1),\
            `unix_perm` INTEGER,\
            `size` INTEGER,\
            `path` VARCHAR(250));",
        ),
        indexes: vec![(
            "files_bundle_uuid_idx",
            "CREATE INDEX IF NOT EXISTS `files_bundle_uuid_idx` ON `files` (`bundle_uuid`);",
        )],
    });
    v1_schema.push(SchemaItem {
        table: (
            "blobs",
            "CREATE TABLE IF NOT EXISTS `blobs` (\
            `blob_uuid` CHAR(36),\
            `size` INTEGER,\
            `status` INTEGER);",
        ),
        indexes: vec![
            (
                "blobs_blob_uuid_idx",
                "CREATE INDEX IF NOT EXISTS `blobs_blob_uuid_idx` ON `blobs` (`blob_uuid`);",
            ),
            (
                "blobs_status_idx",
                "CREATE INDEX IF NOT EXISTS `blobs_status_idx` ON `blobs` (`status`);",
            ),
        ],
    });
    v1_schema.push(SchemaItem {
        table: (
            "blocks",
            "CREATE TABLE IF NOT EXISTS `blocks` (\
            `blob_uuid` CHAR(36),\
            `file_uuid` CHAR(36),\
            `block_num` INTEGER);",
        ),
        indexes: vec![
            (
                "blocks_file_uuid_idx",
                "CREATE INDEX IF NOT EXISTS `blocks_file_uuid_idx` ON `blocks` (`file_uuid`);",
            ),
            (
                "blocks_blob_uuid_idx",
                "CREATE INDEX IF NOT EXISTS `blocks_blob_uuid_idx` ON `blocks` (`blob_uuid`);",
            ),
        ],
    });
    v1_schema.push(SchemaItem{
        table: ("basic_metadata", "CREATE TABLE IF NOT EXISTS `basic_metadata` (\
            `subject_uuid` CHAR(36),\
            `predicate` VARCHAR(250),\
            `value` TEXT);"),
        indexes: vec![
            ("basic_metadata_subject_uuid_idx", "CREATE INDEX IF NOT EXISTS `basic_metadata_subject_uuid_idx` ON `basic_metadata` (`subject_uuid`);"),
            ("basic_metadata_predicate_idx", "CREATE INDEX IF NOT EXISTS `basic_metadata_predicate_idx` ON `basic_metadata` (`predicate`);")
        ]
    });
    v1_schema.push(SchemaItem{
        table: ("inode", "CREATE TABLE IF NOT EXISTS `inode` (\
            `inode_num` INTEGER PRIMARY KEY, /* the value is actually u64 but may be preented as i64 */\
            `object_uuid` CHAR(36),\
            `object_type` VARCHAR(20),\
            `path` VARCHAR(250)\
        ) WITHOUT ROWID;"),
        indexes: vec![
            ("inode_inode_num_idx", "CREATE INDEX IF NOT EXISTS `inode_inode_num_idx` ON `inode` (`inode_num`);"),
            ("inode_object_uuid_idx", "CREATE INDEX IF NOT EXISTS `inode_object_uuid_idx` ON `inode` (`object_uuid`);")
        ]
    });

    for item in v1_schema {
        let table = item.table;
        debug!("Adding table {} with code: {}", table.0, table.1);
        let res = conn.execute(&table.1, params![]);
        if let Err(err) = res {
            error!("Failed to create {} table: {:?}", table.0, err);
            trace!("-{} -> {:?}", stringify!(schema_upgrade_to_v1), err);
            return Err(err);
        }
        for index in item.indexes {
            debug!("Adding index {} with code: {}", index.0, index.1);
            let res = conn.execute(&index.1, params![]);
            if let Err(err) = res {
                error!("Failed to create {} index: {:?}", index.0, err);
                trace!("-{} -> {:?}", stringify!(schema_upgrade_to_v1), err);
                return Err(err);
            }
        }
    }

    let new_schema_version = 1;
    let res = conn.execute(
        "INSERT INTO `app_config` (`key`, `value_num`) VALUES ('schema_version', ?1);",
        params![new_schema_version],
    );
    match res {
        Ok(_) => {
            debug!("Just set schema_version to {}", new_schema_version);
            trace!("-{} -> Ok", stringify!(schema_upgrade_to_v1));
            Ok(())
        }
        Err(err) => {
            error!(
                "Failed to set schema_version to {}, {:?}",
                new_schema_version, err
            );
            trace!("-{} -> {:?}", stringify!(schema_upgrade_to_v1), err);
            Err(err)
        }
    }
}

fn get_schema_version(conn: &Connection) -> SQLResult<i32> {
    trace!("+{}", stringify!(get_schema_version));
    let res: rusqlite::Result<i32> = conn.query_row(
        "SELECT `value_num` FROM `app_config` WHERE `key` = 'schema_version'",
        params![],
        |row| row.get(0),
    );
    let schema_version = match res {
        Ok(v) => v,
        Err(err) => {
            if err == SQLError::QueryReturnedNoRows {
                0
            } else {
                error!("Failed to get schema_version from database: {:?}", err);
                trace!("-{} -> {:?}", stringify!(get_schema_version), err);
                return Err(err);
            }
        }
    };
    debug!("Schema version: {:?}", schema_version);
    trace!(
        "-{} -> Ok({})",
        stringify!(get_schema_version),
        schema_version
    );
    Ok(schema_version)
}

pub fn upgrade_schema(conn: &Connection) -> SQLResult<()> {
    trace!("+{}", stringify!(upgrade_schema));
    // Ensure app_config table exists
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS `app_config` (`key` TEXT PRIMARY KEY, `value_txt` TEXT, `value_num` INTEGER);",
        params![],
    );
    if let Err(err) = res {
        error!("Failed to create app_config table: {:?}", err);
        trace!("-{} -> {:?}", stringify!(upgrade_schema), err);
        return Err(err);
    }

    // Upgrade schema as much as needed
    let mut safety_counter = 0;
    loop {
        let schema_version = get_schema_version(conn)?;
        match schema_version {
            0 => schema_upgrade_to_v1(conn)?,
            _ => break,
        }
        if safety_counter > 100 {
            let msg = "failed to upgrade schema to current version";
            error!("{}", msg);
            panic!(msg);
        }
        safety_counter += 1;
    }
    trace!("-{} -> Ok", stringify!(upgrade_schema));
    Ok(())
}
