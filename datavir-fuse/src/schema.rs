use crate::inode_record::set_inode_counter;
use crate::inode_record::INODE_MIN;
use crate::inode_record::{
    INODE_ALL_BUNDLES_DIR, INODE_ALL_FILTERS_DIR, INODE_CONFIG, INODE_ROOT, INODE_SOCKET,
    INODE_STATUS_FILE, INODE_TRASH_DIR, INODE_VOLUMES_DIR,
};
use crate::prelude::*;
use core::sync::atomic::AtomicU64;
use rusqlite::config::DbConfig;

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
    v1_schema.push(SchemaItem {
        table: (
            "node_meta",
            "CREATE TABLE IF NOT EXISTS `node_meta` (\
            `inode` INTEGER PRIMARY KEY ASC,\
            `main_parent` INTEGER NOT NULL,\
            `obj_uuid` CHAR(36),\
            `obj_type` VARCHAR(20),\
            `file_type` VARCHAR(1),\
            `mtime` INTEGER DEFAULT (strftime('%s','now')) NOT NULL,\
            `ctime` INTEGER DEFAULT (strftime('%s','now')) NOT NULL,\
            `crtime` INTEGER DEFAULT (strftime('%s','now')) NOT NULL\
        ) WITHOUT ROWID;",
        ),
        indexes: vec![
            (
                "node_meta_inode_idx",
                "CREATE INDEX IF NOT EXISTS `node_meta_inode_idx` ON `node_meta` (`inode`);",
            ),
            (
                "node_meta_obj_uuid_idx",
                "CREATE INDEX IF NOT EXISTS `node_meta_obj_uuid_idx` ON `node_meta` (`obj_uuid`);",
            ),
        ],
    });
    v1_schema.push(SchemaItem{
        table: ("node_names", "CREATE TABLE IF NOT EXISTS `node_name` (\
            `inode` INTEGER NOT NULL,\
            `parent` INTEGER, /* parent can be null on purpose, it may help with bundle structure */\
            `hidden` BOOLEAN DEFAULT 0 NOT NULL,\
            `name` VARCHAR(250) NOT NULL\
        )"),
        indexes: vec![
            ("node_names_inode_idx", "CREATE INDEX IF NOT EXISTS `node_names_inode_idx` ON `node_name` (`inode`);"),
            ("node_names_parent_idx", "CREATE INDEX IF NOT EXISTS `node_names_parent_idx` ON `node_name` (`parent`);"),
            ("node_names_uniq_idx", "CREATE UNIQUE INDEX IF NOT EXISTS `node_names_uniq_idx` ON `node_name` (`inode`, `parent`, `name`);")
        ]
    });
    v1_schema.push(SchemaItem {
        table: (
            "node_view",
            "CREATE VIEW IF NOT EXISTS `node_view` AS SELECT\
            `node_meta`.`inode` AS `inode`,\
            `node_meta`.`main_parent` AS `main_parent`,\
            `node_meta`.`obj_uuid` AS `obj_uuid`,\
            `node_meta`.`obj_type` AS `obj_type`,\
            `node_meta`.`file_type` AS `file_type`,\
            `node_meta`.`mtime` AS `mtime`,\
            `node_meta`.`ctime` AS `ctime`,\
            `node_meta`.`crtime` AS `crtime`,\
            `node_name`.`parent` AS `parent`,\
            `node_name`.`hidden` AS `hidden`,\
            `node_name`.`name` AS `name`\
            FROM `node_meta` LEFT JOIN `node_name` ON (`node_meta`.`inode` = `node_name`.`inode`)",
        ),
        indexes: vec![],
    });
    v1_schema.push(SchemaItem {
        table: (
            "node_view_nlink",
            "CREATE VIEW IF NOT EXISTS `node_view_nlink` AS SELECT\
            `node_meta`.`inode` AS `inode`,\
            `node_meta`.`main_parent` AS `main_parent`,\
            COUNT(`node_name`.`name`)  AS `nlink`,\
            `node_meta`.`obj_uuid` AS `obj_uuid`,\
            `node_meta`.`obj_type` AS `obj_type`,\
            `node_meta`.`file_type` AS `file_type`,\
            `node_meta`.`mtime` AS `mtime`,\
            `node_meta`.`ctime` AS `ctime`,\
            `node_meta`.`crtime` AS `crtime`,\
            `node_name`.`parent` AS `parent`,\
            `node_name`.`hidden` AS `hidden`\
            FROM `node_meta` LEFT JOIN `node_name` ON (`node_meta`.`inode` = `node_name`.`inode`)\
            GROUP BY `node_meta`.`inode`",
        ),
        indexes: vec![],
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

fn reserve_inodes(conn: &Connection) -> SQLResult<()> {
    trace!("+{}", stringify!(reserve_inodes));
    let res: rusqlite::Result<i32> = conn.query_row(
        "SELECT COUNT() FROM `node_meta` WHERE `inode` < ?1 AND `obj_type` != 'R'",
        params![INODE_MIN],
        |row| row.get(0),
    );
    let n_non_reserved = match res {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to get count of inodes: {:?}", err);
            trace!("-{} -> {:?}", stringify!(reserve_inodes), err);
            return Err(err);
        }
    };

    if n_non_reserved > 0 {
        let msg = format!(
            "There are {} non-special inodes below inode number {}. This MUST NEVER happen!",
            n_non_reserved, INODE_MIN
        );
        error!("{}", msg);
        panic!(msg);
    }

    for inode in 0..INODE_MIN {
        trace!("Reserving inode number {}", inode);
        conn.execute(
            "INSERT OR REPLACE INTO `node_meta` (`inode`, `main_parent`, `obj_uuid`, `obj_type`) VALUES \
            (?1, 1, '00000000-0000-0000-0000-000000000000', 'R')",
            params![inode],
        )?;
    }

    trace!("Reserving root inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'D' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving config inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'F' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_CONFIG)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, 'datavir.toml')",
        params![u64_to_i64(INODE_CONFIG), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving socket inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'S' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_SOCKET)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, '.datavir.socket')",
        params![u64_to_i64(INODE_SOCKET), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving status inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'F' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_STATUS_FILE)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, '.datavir.status')",
        params![u64_to_i64(INODE_STATUS_FILE), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving volumes inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'D' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_VOLUMES_DIR)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, 'Volumes')",
        params![u64_to_i64(INODE_VOLUMES_DIR), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving all bundles inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'D' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_ALL_BUNDLES_DIR)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, 'All Bundles')",
        params![u64_to_i64(INODE_ALL_BUNDLES_DIR), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving all filters inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'D' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_ALL_FILTERS_DIR)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, 'All Filters')",
        params![u64_to_i64(INODE_ALL_FILTERS_DIR), u64_to_i64(INODE_ROOT)],
    )?;
    trace!("Reserving trash inode");
    conn.execute(
        "UPDATE `node_meta` SET\
        `file_type` = 'D' WHERE `inode` = ?1",
        params![u64_to_i64(INODE_TRASH_DIR)],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO `node_name` (`inode`, `parent`, `hidden`, `name`) VALUES \
        (?1, ?2, 0, 'Trash')",
        params![u64_to_i64(INODE_TRASH_DIR), u64_to_i64(INODE_ROOT)],
    )?;

    debug!("Reserved special inodes");
    trace!("-{} -> Ok", stringify!(reserve_inodes));
    Ok(())
}

fn upgrade_schema(conn: &Connection, inode_next: &AtomicU64) -> SQLResult<()> {
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

    // Ensure certain inode numbers are reserved
    reserve_inodes(conn)?;
    // Adjust inode counter
    set_inode_counter(conn, inode_next)?;

    trace!("-{} -> Ok", stringify!(upgrade_schema));
    Ok(())
}

fn set_conn_options(conn: &Connection) -> SQLResult<()> {
    trace!("+{}", stringify!(set_conn_options));
    let opts = vec![
        // We don't need foreign keys
        (
            DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY,
            "SQLITE_DBCONFIG_ENABLE_FKEY",
            false,
        ),
        // We do need triggers
        (
            DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER,
            "SQLITE_DBCONFIG_ENABLE_TRIGGER",
            true,
        ),
        // We don't use full text search
        (
            DbConfig::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER,
            "SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER",
            false,
        ),
        // Enable checkpoints (yes, it is `false` to enable)
        (
            DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
            "SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE",
            false,
        ),
        // Enable "stable" query times
        (
            DbConfig::SQLITE_DBCONFIG_ENABLE_QPSG,
            "SQLITE_DBCONFIG_ENABLE_QPSG",
            true,
        ),
        // Add some protection against mistakes
        (
            DbConfig::SQLITE_DBCONFIG_DEFENSIVE,
            "SQLITE_DBCONFIG_DEFENSIVE",
            true,
        ),
    ];
    for opt in opts {
        debug!("Setting {} to {:?}", opt.1, opt.2);
        if let Err(err) = conn.set_db_config(opt.0, opt.2) {
            error!("Failed to set {}: {:?}", opt.1, err);
            trace!("-{} -> {:?}", stringify!(set_conn_options), err);
            return Err(err);
        }
    }
    trace!("-{} -> Ok", stringify!(set_conn_options));
    Ok(())
}

pub fn open_database(db_path: &Path, inode_next: &AtomicU64) -> SQLResult<Connection> {
    trace!("+{}(db_path={:?})", stringify!(open_database), db_path);
    match Connection::open(db_path) {
        Ok(conn) => {
            set_conn_options(&conn)?;
            upgrade_schema(&conn, inode_next)?;
            trace!(
                "-{}(db_path={:?}) -> Ok",
                stringify!(open_database),
                db_path
            );
            return Ok(conn);
        }
        Err(err) => {
            error!("Failed to open database at {:?}: {:?}", db_path, err);
            trace!(
                "-{}(db_path={:?}) -> {:?}",
                stringify!(open_database),
                db_path,
                err
            );
            return Err(err);
        }
    }
}
