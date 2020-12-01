pub use log::{debug, error, info, trace, warn};

pub use std::fs;
pub use std::io::Error as IOError;
pub use std::io::ErrorKind as IOErrorKind;
pub use std::io::Result as IOResult;
pub use std::path::{Path, PathBuf};
pub use std::time::SystemTimeError;

pub use rusqlite::params;
pub use rusqlite::Connection;
pub use rusqlite::Error as SQLError;
pub use rusqlite::Result as SQLResult;
pub use rusqlite::Transaction;

pub use crate::object_type::ObjectType;

#[derive(Debug)]
pub enum DVError {
    SQLError(SQLError),
    IOError(IOError),
    DirNotClean(PathBuf),
    SystemTimeError(SystemTimeError),
    TimeConversionErrorFromSecs(u64),
}

pub type DVResult<T> = Result<T, DVError>;

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
