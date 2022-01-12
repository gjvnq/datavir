pub use log::{debug, error, info, trace, warn};

pub use std::io::Error as IOError;
pub use std::io::ErrorKind as IOErrorKind;
pub use std::io::Result as IOResult;
pub use std::time::SystemTimeError;

pub use rusqlite::Connection as SQLConnection;
pub use rusqlite::Error as SQLError;
pub use rusqlite::Result as SQLResult;
pub use rusqlite::Transaction as SQLTransaction;

pub use uuid::Uuid;

pub const DATAVIT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
#[allow(dead_code)]
pub enum DVError {
    SQLError(SQLError),
    IOError(IOError),
    SystemTimeError(SystemTimeError),
    TimeConversionErrorFromSecs(u64),
    UuidParseError(String),
    NotImplemented,
    NoMoreResults,
}

pub type DVResult<T> = Result<T, DVError>;

impl DVError {
    #[allow(dead_code)]
    pub fn is_not_found(&self) -> bool {
        match self {
            DVError::SQLError(err) => is_sql_err_not_found(err),
            DVError::IOError(err) => match err.kind() {
                std::io::ErrorKind::NotFound => true,
                _ => false,
            },
            _ => false,
        }
    }
}

#[allow(dead_code)]
pub fn is_sql_err_not_found(err: &SQLError) -> bool {
    match err {
        rusqlite::Error::QueryReturnedNoRows => true,
        _ => false,
    }
}

impl std::convert::From<SystemTimeError> for DVError {
    fn from(err: SystemTimeError) -> Self {
        DVError::SystemTimeError(err)
    }
}

impl std::convert::From<IOError> for DVError {
    fn from(err: IOError) -> Self {
        DVError::IOError(err)
    }
}

impl std::convert::From<SQLError> for DVError {
    fn from(err: SQLError) -> Self {
        DVError::SQLError(err)
    }
}

#[allow(dead_code)]
pub fn str_to_uuid(val: &str) -> DVResult<Uuid> {
    match Uuid::parse_str(&val) {
        Ok(v) => Ok(v),
        Err(_) => Err(DVError::UuidParseError(val.to_string())),
    }
}

#[allow(dead_code)]
pub fn fuck_ref<'a, T>(ptr: &T) -> &'a T {
    unsafe { &*(ptr as *const T) }
}

#[allow(dead_code)]
pub fn fuck_mut<'a, T>(ptr: &mut T) -> &'a mut T {
    unsafe { &mut *(ptr as *mut T) }
}

#[allow(dead_code)]
#[inline]
pub fn i64_to_u64(num: i64) -> u64 {
    unsafe { std::mem::transmute(num) }
}

#[allow(dead_code)]
#[inline]
pub fn u64_to_i64(num: u64) -> i64 {
    unsafe { std::mem::transmute(num) }
}

#[allow(dead_code)]
#[inline]
pub fn i32_to_u32(num: i32) -> u32 {
    unsafe { std::mem::transmute(num) }
}

#[allow(dead_code)]
#[inline]
pub fn u32_to_i32(num: u32) -> i32 {
    unsafe { std::mem::transmute(num) }
}