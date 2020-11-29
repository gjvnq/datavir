use crate::node_type::NodeType;
use log;
use rusqlite::{params, Connection};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

#[allow(unused_imports)]
use log::{debug, info, trace, warn, error};


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

pub trait INodeRegisterable: std::fmt::Debug {
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

#[inline]
fn i64_to_u64(num: i64) -> u64 {
    unsafe { std::mem::transmute(num) }
}

#[inline]
fn u64_to_i64(num: u64) -> i64 {
    unsafe { std::mem::transmute(num) }
}

impl INodeRecord {
    fn new(num: i64, uuid: Uuid, kind: NodeType, path: String) -> Self {
        INodeRecord {
            inode_num: i64_to_u64(num),
            object_uuid: uuid,
            object_type: kind,
            path: path,
        }
    }
}

#[allow(dead_code)]
fn set_inode_counter(conn: Connection) -> Result<(), rusqlite::Error> {
    trace!("+{}", stringify!(set_inode_counter));
    let inode_num = get_highest_inode(conn);
    if let Err(err) = inode_num {
        error!("Failed to set INODE_NEXT: {:?}", err);
        trace!("-{} -> {:?}", stringify!(set_inode_counter), err);
        return Err(err);
    }
    let inode_num = inode_num.unwrap();
    INODE_NEXT.store(inode_num + 1, Ordering::SeqCst);
    debug!("Set INODE_NEXT to {}", inode_num);
    trace!("-{}", stringify!(set_inode_counter));
    return Ok(());
}

fn get_highest_inode(conn: Connection) -> Result<u64, rusqlite::Error> {
    trace!("+{}", stringify!(get_highest_inode));
    // This weird code is because I want u64 but SQLite only stores i64
    let res_max = conn.query_row("SELECT MAX(`inode_num`) FROM `inode`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_max {
        error!("Failed to get MAX(`inode_num`): {:?}", err);
        return Err(err);
    }
    let res_min = conn.query_row("SELECT MIN(`inode_num`) FROM `inode`", params![], |row| {
        Ok(row.get(0)?)
    });
    if let Err(err) = res_min {
        error!("Failed to get MIN(`inode_num`): {:?}", err);
        return Err(err);
    }

    let a = i64_to_u64(res_max.unwrap());
    let b = i64_to_u64(res_min.unwrap());
    if a > b {
        trace!("-{} -> {}", stringify!(get_highest_inode), a);
        Ok(a)
    } else {
        trace!("-{} -> {}", stringify!(get_highest_inode), b);
        Ok(b)
    }
}

#[allow(dead_code)]
fn inode_add(obj: &dyn INodeRegisterable, conn: Connection) -> Result<(), rusqlite::Error> {
    trace!("+{}(obj={:?})", stringify!(inode_add), obj);
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
        Ok(_) => {
            trace!("-{}(obj={:?}) -> Ok", stringify!(inode_add), obj);
            Ok(())
        },
        Err(err) => {
            error!("Failed to add new inode to database: {:?}", err);
            trace!("-{}(obj={:?}) -> {:?}", stringify!(inode_add), obj, err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_get(inode_num: u64, conn: Connection) -> Result<INodeRecord, rusqlite::Error> {
    trace!("+{}(inode_num={})", stringify!(inode_get), inode_num);
    let res = conn.query_row(
        "SELECT `inode_num`, `object_uuid`, `object_type`, `path` FROM `inode` WHERE `inode_num` = ?1",
        params![u64_to_i64(inode_num)],
        |row| Ok(INodeRecord::new(row.get(0)?, row.get(1)?, row.get(2)?,row.get(3)?))
    );
    match res {
        Ok(v) => {
            trace!("-{}(inode_num={}) -> {:?}", stringify!(inode_get), inode_num, v);
            Ok(v)
        },
        Err(err) => {
            error!("Failed to get inode {} from database: {:?}", inode_num, err);
            trace!("-{}(inode_num={}) -> {:?}", stringify!(inode_get), inode_num, err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_set_path(inode_num: u64, path: &String, conn: Connection) -> Result<(), rusqlite::Error> {
    trace!("+{}(inode_num={},path={:?})", stringify!(inode_set_path), inode_num, path);
    let res = conn.execute(
        "UPDATE `inode` SET `path` = ?1 WHERE `inode_num` = ?2",
        params![path, u64_to_i64(inode_num)],
    );
    match res {
        Ok(_) => {
            trace!("+{}(inode_num={},path={}) -> Ok", stringify!(inode_set_path), inode_num, path);
            Ok(())
        },
        Err(err) => {
            error!(
                "Failed to set inode {} path to '{}' from database: {:?}",
                inode_num,
                path,
                err
            );
            trace!("-{}(inode_num={},path={}) -> {:?}", stringify!(inode_set_path), inode_num, path, err);
            Err(err)
        }
    }
}

#[allow(dead_code)]
fn inode_del(inode_num: u64, conn: Connection) -> Result<(), rusqlite::Error> {
    trace!("+{}(inode_num={})", stringify!(inode_del), inode_num);
    let res = conn.execute(
        "DELETE FROM `inode` WHERE `inode_num` = ?1",
        params![u64_to_i64(inode_num)],
    );
    match res {
        Ok(_) => {
            trace!("-{}(inode_num={}) -> Ok", stringify!(inode_del), inode_num);
            Ok(())
        },
        Err(err) => {
            error!(
                "Failed to delete inode {} from database: {:?}",
                inode_num,
                err
            );
            trace!("-{}(inode_num={}) -> {:?}", stringify!(inode_del), inode_num, err);
            Err(err)
        }
    }
}
