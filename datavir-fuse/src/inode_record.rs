use crate::node_type::NodeType;
use log;
use rusqlite::{params, Connection};
use std::sync::atomic::{AtomicU64, Ordering};

use uuid::Uuid;

#[allow(dead_code)]
const INODE_ROOT: u64 = 1;
#[allow(dead_code)]
const INODE_CONFIG: u64 = 2;
#[allow(dead_code)]
const INODE_SOCKET: u64 = 3;
#[allow(dead_code)]
const INODE_BUNDLES_TXT: u64 = 4;
#[allow(dead_code)]
const INODE_FILTERS_DIR: u64 = 5;
#[allow(dead_code)]
const INODE_ALL_BUNDLES_DIR: u64 = 6;
#[allow(dead_code)]
const INODE_BUNDLES_DIR: u64 = 7;

const INODE_MIN: u64 = 64;
static INODE_NEXT: AtomicU64 = AtomicU64::new(INODE_MIN);

pub trait INodeRegisterable {
    fn get_uuid(&self) -> Uuid;
    fn get_node_type(&self) -> NodeType;
    fn get_path(&self) -> String;
}

#[derive(Debug)]
struct INodeRecord {
    inode_num: u64,
    object_uuid: Uuid,
    object_type: NodeType,
    path: String,
}

fn i64_to_u64(num: i64) -> u64 {
    unsafe{
        std::mem::transmute(num)
    }
}

fn u64_to_i64(num: u64) -> i64 {
    unsafe{
        std::mem::transmute(num)
    }
}

impl INodeRecord {
    fn new(num: i64, uuid: Uuid, kind: NodeType, path: String) -> Self {
        INodeRecord{
            inode_num: i64_to_u64(num),
            object_uuid: uuid,
            object_type: kind,
            path: path
        }
    }
}

#[allow(dead_code)]
fn set_inode_counter(conn: Connection) -> Result<(), rusqlite::Error> {
    let inode_num = get_highest_inode(conn);
    if let Err(err) = inode_num {
        log::error!("Failed to set INODE_NEXT: {:?}", err);
        return Err(err);
    }
    INODE_NEXT.store(inode_num.unwrap()+1, Ordering::SeqCst);
    return Ok(());
}

fn get_highest_inode(conn: Connection) -> Result<u64, rusqlite::Error> {
    // This weird code is because I want u64 but SQLite only stores i64
    let res_max = conn.query_row(
        "SELECT MAX(`inode_num`) FROM `inode`",
        params![],
        |row| Ok(row.get(0)?)
    );
    if let Err(err) = res_max {
        log::error!("Failed to get MAX(`inode_num`): {:?}", err);
        return Err(err);
    }
    let res_min = conn.query_row(
        "SELECT MIN(`inode_num`) FROM `inode`",
        params![],
        |row| Ok(row.get(0)?)
    );
    if let Err(err) = res_min {
        log::error!("Failed to get MIN(`inode_num`): {:?}", err);
        return Err(err);
    }

    let a = i64_to_u64(res_max.unwrap());
    let b = i64_to_u64(res_min.unwrap());
    if a > b {
        Ok(a)
    } else {
        Ok(b)
    }
}

#[allow(dead_code)]
fn inode_add(obj: &dyn INodeRegisterable, conn: Connection) -> Result<(), rusqlite::Error> {
    let inode_num = INODE_NEXT.fetch_add(1, Ordering::SeqCst);
    let res = conn.execute(
        "INSERT INTO `inode` (`inode_num`, `object_uuid`, `object_type`, `path`) (?1, ?2, ?3, ?4)",
        params![
            u64_to_i64(inode_num),
            obj.get_uuid(),
            obj.get_node_type(),
            obj.get_path()
        ],
    );
    match res {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!("Failed to add new inode to database: {:?}", err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_get(inode_num: u64, conn: Connection) -> Result<INodeRecord, rusqlite::Error> {
    let res = conn.query_row(
        "SELECT `inode_num`, `object_uuid`, `object_type`, `path` FROM `inode` WHERE `inode_num` = ?1",
        params![u64_to_i64(inode_num)],
        |row| Ok(INodeRecord::new(row.get(0)?, row.get(1)?, row.get(2)?,row.get(3)?))
    );
    match res {
        Ok(v) => Ok(v),
        Err(err) => {
            log::error!("Failed to get inode {} from database: {:?}", inode_num, err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_set_path(inode_num: u64, path: &String, conn: Connection) -> Result<(), rusqlite::Error> {
    let res = conn.execute(
        "UPDATE `inode` SET `path` = ?1 WHERE `inode_num` = ?2",
        params![path, u64_to_i64(inode_num)]);
    match res {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!("Failed to set inode {} path to '{}' from database: {:?}", inode_num, path, err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_del(inode_num: u64, conn: Connection) -> Result<(), rusqlite::Error> {
    let res = conn.execute(
        "DELETE FROM `inode` WHERE `inode_num` = ?1",
        params![u64_to_i64(inode_num)]);
    match res {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!("Failed to delete inode {} from database: {:?}", inode_num, err);
            Err(err)
        }
    }
}
