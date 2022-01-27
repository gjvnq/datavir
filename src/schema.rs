use crate::prelude::*;
use rusqlite::config::DbConfig;

#[derive(Debug)]
struct SchemaItem<'a> {
    name: &'a str,
    kind: &'a str,
    code: &'a str
}

fn schema_upgrade_to_v1(conn: &SQLConnection) -> SQLResult<()> {
    let trace_msg = format!("{}()", function!());
    trace!("+{}", trace_msg);
    let mut v1_schema: Vec<SchemaItem> = vec![];
    v1_schema.push(SchemaItem {
        name: "filenode",
        kind: "table",
        code: "CREATE TABLE `filenode` (\
            `inode_num` INTEGER PRIMARY KEY,\
            `node_uuid` NOT NULL UNIQUE,\
            `parent_uuid` NOT NULL,\
            `filename` NOT NULL,\
            `contents` NULL,\
            `super_hidden` DEFAULT 0,\
            `changed_at` NOT NULL,\
            `created_at` NOT NULL\
            );",
    });
    
    for item in v1_schema {
        debug!("Adding {} {} with code: {}", item.kind, item.name, item.code);
        let res = conn.execute(&item.code, params![]);
        if let Err(err) = res {
            error!("Failed to create {} {}: {:?}", item.name, item.kind, err);
            trace!("-{} -> {:?}", trace_msg, err);
            return Err(err);
        }
    }

    let new_schema_version = 1;
    let res = conn.execute(
        "INSERT INTO `app_config` (`key`, `value`) VALUES ('schema_version', ?1);",
        params![new_schema_version],
    );
    match res {
        Ok(_) => {
            debug!("Just set schema_version to {}", new_schema_version);
            trace!("-{} -> Ok", trace_msg);
            Ok(())
        }
        Err(err) => {
            error!(
                "Failed to set schema_version to {}, {:?}",
                new_schema_version, err
            );
            trace!("-{} -> {:?}", trace_msg, err);
            Err(err)
        }
    }
}

fn get_schema_version(conn: &SQLConnection) -> SQLResult<i32> {
    let trace_msg = format!("{}()", function!());
    trace!("+{}", trace_msg);

    let res: rusqlite::Result<i32> = conn.query_row(
        "SELECT `value` FROM `app_config` WHERE `key` = 'schema_version'",
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
                trace!("-{} -> {:?}", trace_msg, err);
                return Err(err);
            }
        }
    };
    debug!("Schema version: {:?}", schema_version);
    trace!(
        "-{} -> Ok({})",
        trace_msg,
        schema_version
    );
    Ok(schema_version)
}

fn upgrade_schema(conn: &SQLConnection) -> SQLResult<()> {
    let trace_msg = format!("{}()", function!());
    trace!("+{}", trace_msg);
    // Ensure app_config table exists
    let res = conn.execute(
        "CREATE TABLE IF NOT EXISTS `app_config` (`key` NOT NULL, `value`);",
        params![],
    );
    if let Err(err) = res {
        error!("Failed to create app_config table: {:?}", err);
        trace!("-{} -> {:?}", trace_msg, err);
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
            panic!("{}", msg);
        }
        safety_counter += 1;
    }

    trace!("-{} -> Ok", trace_msg);
    Ok(())
}

fn set_conn_options(conn: &SQLConnection) -> SQLResult<()> {
    let trace_msg = format!("{}()", function!());
    trace!("+{}", trace_msg);
    let opts = vec![
        // We need foreign keys
        (
            DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY,
            "SQLITE_DBCONFIG_ENABLE_FKEY",
            true,
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
            trace!("-{} -> {:?}", trace_msg, err);
            return Err(err);
        }
    }
    trace!("-{} -> Ok", trace_msg);
    Ok(())
}

pub fn open_database(db_path: &Path) -> SQLResult<SQLConnection> {
    let trace_msg = format!("{}", function!());
    trace!("+{}(db_path={:?})", trace_msg, db_path);
    match SQLConnection::open(db_path) {
        Ok(conn) => {
            set_conn_options(&conn)?;
            upgrade_schema(&conn)?;
            trace!(
                "-{}(db_path={:?}) -> Ok",
                trace_msg,
                db_path
            );
            return Ok(conn);
        }
        Err(err) => {
            error!("Failed to open database at {:?}: {:?}", db_path, err);
            trace!(
                "-{}(db_path={:?}) -> {:?}",
                trace_msg,
                db_path,
                err
            );
            return Err(err);
        }
    }
}
