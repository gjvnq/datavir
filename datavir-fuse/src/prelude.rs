pub use log::{debug, error, info, trace, warn};

pub use std::fs;
pub use std::io::Error as IOError;
pub use std::io::ErrorKind as IOErrorKind;
pub use std::io::Result as IOResult;
pub use std::path::{Path, PathBuf};

pub use rusqlite::params;
pub use rusqlite::Connection;
pub use rusqlite::Error as SQLError;
pub use rusqlite::Result as SQLResult;
pub use rusqlite::Transaction;

#[derive(Debug)]
pub enum DVError {
    SQLError(SQLError),
    IOError(IOError),
    DirNotClean(PathBuf),
}

pub type DVResult<T> = Result<T, DVError>;
