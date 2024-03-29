pub use log::{debug, error, info, trace, warn};

pub use core::cell::RefCell;
pub use std::fs;
pub use std::io::Error as IOError;
pub use std::io::ErrorKind as IOErrorKind;
pub use std::io::Result as IOResult;
pub use std::mem::MaybeUninit;
pub use std::panic;
pub use std::path::{Path, PathBuf};
pub use std::ptr::NonNull;
pub use std::sync::Arc;
pub use std::sync::Mutex;
pub use std::time::SystemTimeError;

pub use rusqlite::params;
pub use rusqlite::Connection;
pub use rusqlite::Error as SQLError;
pub use rusqlite::Result as SQLResult;
pub use rusqlite::Transaction;

pub use uuid::Uuid;

pub use crate::object_type::ObjectType;

#[derive(Debug)]
pub enum DVError {
    SQLError(SQLError),
    IOError(IOError),
    DirNotClean(PathBuf),
    SystemTimeError(SystemTimeError),
    TimeConversionErrorFromSecs(u64),
    FuseTypeParseError(String),
    NodeNoNum,
    UuidParseError(String),
    NotImplemented,
    NoMoreResults,
}

pub type DVResult<T> = Result<T, DVError>;

impl DVError {
    pub fn is_not_found(&self) -> bool {
        match self {
            DVError::SQLError(err) => is_sql_not_found(err),
            DVError::IOError(err) => match err.kind() {
                std::io::ErrorKind::NotFound => true,
                _ => false,
            },
            _ => false,
        }
    }
}

pub fn is_sql_not_found(err: &SQLError) -> bool {
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

impl std::convert::From<DVError> for libc::c_int {
    fn from(err: DVError) -> Self {
        match err {
            DVError::SQLError(err) => match err {
                SQLError::QueryReturnedNoRows => POSIX_NOT_FOUND,
                _ => POSIX_IO_ERROR,
            },
            DVError::IOError(err) => match err.kind() {
                IOErrorKind::NotFound => POSIX_NOT_FOUND,
                IOErrorKind::PermissionDenied => POSIX_PERMISSION_DENINED,
                IOErrorKind::AlreadyExists => POSIX_FILE_EXISTS,
                _ => POSIX_IO_ERROR,
            },
            _ => POSIX_IO_ERROR,
        }
    }
}

pub fn parse_uuid(val: &str) -> DVResult<Uuid> {
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

#[inline]
pub fn i64_to_u64(num: i64) -> u64 {
    unsafe { std::mem::transmute(num) }
}

#[inline]
pub fn u64_to_i64(num: u64) -> i64 {
    unsafe { std::mem::transmute(num) }
}

#[inline]
pub fn i32_to_u32(num: i32) -> u32 {
    unsafe { std::mem::transmute(num) }
}

#[inline]
pub fn u32_to_i32(num: u32) -> i32 {
    unsafe { std::mem::transmute(num) }
}

// From https://gist.github.com/nelsonsar/549f7167aa2091afafa5
#[allow(dead_code)]
pub const POSIX_NOT_PERMITTED: libc::c_int = libc::EPERM;
#[allow(dead_code)]
pub const POSIX_NOT_FOUND: libc::c_int = libc::ENOENT;
#[allow(dead_code)]
pub const POSIX_NO_SUCH_PROCESS: libc::c_int = libc::ESRCH;
#[allow(dead_code)]
pub const POSIX_INTERRUPTED_SYSCALL: libc::c_int = libc::EINTR;
#[allow(dead_code)]
pub const POSIX_IO_ERROR: libc::c_int = libc::EIO;
#[allow(dead_code)]
pub const POSIX_DEVICE_NOT_CONFIGURED: libc::c_int = libc::ENXIO;
#[allow(dead_code)]
pub const POSIX_ARG_LIST_TOO_LONG: libc::c_int = libc::E2BIG;
#[allow(dead_code)]
pub const POSIX_BAD_FILE_DESCRIPTOR: libc::c_int = libc::EBADF;
#[allow(dead_code)]
pub const POSIX_CANNOT_ALLOC_MEMORY: libc::c_int = libc::ENOMEM;
#[allow(dead_code)]
pub const POSIX_PERMISSION_DENINED: libc::c_int = libc::EACCES;
#[allow(dead_code)]
pub const POSIX_BLOCK_DEVICE_REQUIRED: libc::c_int = libc::ENOTBLK;
#[allow(dead_code)]
pub const POSIX_DEVICE_BUSY: libc::c_int = libc::EBUSY;
#[allow(dead_code)]
pub const POSIX_FILE_EXISTS: libc::c_int = libc::EEXIST;
#[allow(dead_code)]
pub const POSIX_CROSS_DEVICE_LINK: libc::c_int = libc::EXDEV;
#[allow(dead_code)]
pub const POSIX_OPERATION_NOT_SUPPORTED: libc::c_int = libc::ENODEV;
#[allow(dead_code)]
pub const POSIX_NOT_A_DIRECTORY: libc::c_int = libc::ENOTDIR;
#[allow(dead_code)]
pub const POSIX_IS_A_DIRECTORY: libc::c_int = libc::EISDIR;
#[allow(dead_code)]
pub const POSIX_INVALID_ARG: libc::c_int = libc::EINVAL;
#[allow(dead_code)]
pub const POSIX_INAPPROPRIATE_IOCTL: libc::c_int = libc::ENOTTY;
#[allow(dead_code)]
pub const POSIX_TEXT_FILE_BUSY: libc::c_int = libc::ETXTBSY;
#[allow(dead_code)]
pub const POSIX_FILE_TOO_LARGE: libc::c_int = libc::EFBIG;
#[allow(dead_code)]
pub const POSIX_NO_SPACE_LEFT: libc::c_int = libc::ENOSPC;
#[allow(dead_code)]
pub const POSIX_ILLEGAL_SEEK: libc::c_int = libc::ESPIPE;
#[allow(dead_code)]
pub const POSIX_READ_ONLY_FS: libc::c_int = libc::EROFS;
#[allow(dead_code)]
pub const POSIX_TOO_MANY_LINKS: libc::c_int = libc::EMLINK;
#[allow(dead_code)]
pub const POSIX_BROKEN_PIPE: libc::c_int = libc::EPIPE;
#[allow(dead_code)]
pub const POSIX_RESULT_TOO_LARGE: libc::c_int = libc::ERANGE;
#[allow(dead_code)]
pub const POSIX_TEMPORARILY_UNAVAILABLE: libc::c_int = libc::EAGAIN;
#[allow(dead_code)]
pub const POSIX_NO_BUFFER_SPACE: libc::c_int = libc::ENOBUFS;
#[allow(dead_code)]
pub const POSIX_SYMLINK_LOOP: libc::c_int = libc::ELOOP;
#[allow(dead_code)]
pub const POSIX_NO_LOCKS_AVAILABLE: libc::c_int = libc::ENOLCK;
#[allow(dead_code)]
pub const POSIX_NOT_IMPLEMENTED: libc::c_int = libc::ENOSYS;
#[allow(dead_code)]
pub const POSIX_OPERATION_CANCELLED: libc::c_int = libc::ECANCELED;
#[allow(dead_code)]
pub const POSIX_DATA_NOT_FOUND: libc::c_int = libc::ENODATA;
#[allow(dead_code)]
pub const POSIX_BAD_MESSAGE: libc::c_int = libc::EBADMSG;
#[allow(dead_code)]
pub const POSIX_BROKEN_LINK: libc::c_int = libc::ENOLINK;
