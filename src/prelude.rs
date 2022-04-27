pub use log::{log, debug, error, info, trace, warn};

pub use std::fs;
pub use std::io::Error as IOError;
pub use std::io::ErrorKind as IOErrorKind;
pub use std::io::Result as IOResult;
pub use std::path::{Path, PathBuf};
pub use std::time::SystemTimeError;
pub use std::collections::HashMap;
pub use std::pin::Pin;
pub use std::marker::PhantomPinned;
pub use std::sync::{Arc, Mutex};
pub use std::sync::mpsc;

pub use tokio_tungstenite::tungstenite::Error as WSError;
pub use futures_util::{StreamExt, SinkExt};

pub use rusqlite::params;
pub use rusqlite::Connection as SQLConnection;
pub use rusqlite::Error as SQLError;
pub use rusqlite::Result as SQLResult;
pub use rusqlite::Transaction as SQLTransaction;

pub use uuid::Uuid;
pub use uuid::v1::Timestamp as UuidTimestamp;
pub use uuid::v1::Context as UuidContext;

pub use chrono::Utc;
pub use chrono::DateTime;

pub const DEFAULT_WS_ADDR: &str = "127.0.0.1:8081";
pub const DEFAULT_WS_ADDR_URL: &str = "ws://127.0.0.1:8081";

static mut UUID_NODE_ID: [u8;6] = [1, 2, 3, 4, 5, 6];
static mut UUID_CONTEXT: Option<UuidContext> = None;

pub fn new_uuid_at(now: DateTime<Utc>) -> Uuid {
    let unix_sec : u64 = match now.timestamp().try_into() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to get unix time for {}: can't convert to u64. Will use random UUID", now);
            return Uuid::new_v4()
        }
    };
    unsafe {
        let context = UUID_CONTEXT.as_ref().expect("UUID context was not initialized");
        let ts = UuidTimestamp::from_unix(context, unix_sec, now.timestamp_subsec_nanos());
        let ans = Uuid::new_v1(ts, &UUID_NODE_ID);
        match ans {
            Ok(v) => v,
            Err(err) => {
                error!("Failed to get unix time for {}: {}. Will use random UUID", now, err);
                return Uuid::new_v4()
            }
        }
    }
}

pub unsafe fn init_uuid_context() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    if UUID_CONTEXT.is_none() {
        UUID_CONTEXT = Some(UuidContext::new(0));
        UUID_NODE_ID[0] = rng.gen::<u8>();
        UUID_NODE_ID[1] = rng.gen::<u8>();
        UUID_NODE_ID[2] = rng.gen::<u8>();
        UUID_NODE_ID[3] = rng.gen::<u8>();
        UUID_NODE_ID[4] = rng.gen::<u8>();
        UUID_NODE_ID[5] = rng.gen::<u8>();
    }
}


pub const DATAVIR_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const DATAVIR_PKG_PREIX: &str = concat!(env!("CARGO_PKG_NAME"), "::");
pub const DATAVIR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
#[allow(dead_code)]
pub enum DVError {
    SQLError(SQLError),
    IOError(IOError),
    WSError(WSError),
    SystemTimeError(SystemTimeError),
    TimeConversionErrorFromSecs(u64),
    UuidParseError(String),
    DirNotClear(PathBuf),
    MpscRecvError(mpsc::RecvError),
    MpscSendError(String),
    InvalidUrl(String),
    NotImplemented,
    NoMoreResults,
    NotReady(String)
}

pub type DVResult<T> = Result<T, DVError>;

// unsafe impl Send for DVError {}
// unsafe impl Sync for DVError {}

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

impl std::convert::From<WSError> for DVError {
    fn from(err: WSError) -> Self {
        DVError::WSError(err)
    }
}

impl std::convert::From<mpsc::RecvError> for DVError {
    fn from(err: mpsc::RecvError) -> Self {
        DVError::MpscRecvError(err)
    }
}

impl<T> std::convert::From<mpsc::SendError<T>> for DVError {
    fn from(err: mpsc::SendError<T>) -> Self {
        DVError::MpscSendError(format!("{:?}", err))
    }
}

impl<T> std::convert::From<DVError> for DVResult<T> {
    fn from(err: DVError) -> Self {
        Err(err)
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

#[allow(unused_macros)]
macro_rules! error_or_warn {
    (target: $target:expr, $is_err:expr, $($arg:tt)+) => (
        if $is_err {
            error!(target: $target, $($arg)+)
        } else {
            warn!(target: $target, $($arg)+)
        }
    );
    ($is_err:expr, $($arg:tt)+) => (
        if $is_err {
            error!($($arg)+);
        } else {
            warn!($($arg)+);
        }
    )
}

#[allow(unused_macros)]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        let clean_name = &name[..name.len() - 3];

        match clean_name.rfind("::") {
            None => &clean_name,
            Some(end1) => match clean_name[..end1].rfind("::") {
                None => &clean_name[end1+2..],
                Some(end2) => &clean_name[end2+2..]
            }
        }
    }};
}


pub fn default_logging_setup(verbosity: u64, log_filepath: &str) -> Result<(), fern::InitError> {
    use fern::colors::{Color, ColoredLevelConfig};

    let colors = ColoredLevelConfig::new()
        .debug(Color::Magenta)
        .trace(Color::BrightBlack);

    let base_config = fern::Dispatch::new();
    let stdout_level = match verbosity {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let file_level = match verbosity {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };

    let file_config = fern::Dispatch::new()
        .level(file_level)
        .format(move |out, message, record| {
            let mut module_or_target = record.module_path().unwrap_or(record.target());
            if module_or_target.starts_with(DATAVIR_PKG_PREIX) {
                module_or_target = DATAVIR_PKG_NAME
            }
            
            // TODO: better way to decide what to show in the file path
            let file = record.file().unwrap_or("?");
            let file = match file.len() {
                n if n > 20 => &file[file.len() - 20..],
                _ => &file,
            };

            out.finish(format_args!(
                "{color_code}{date}[{level: <5}][{target}][{file}:{line: <4}] {message}\x1B[0m",
                color_code = format_args!(
                    "\x1B[{}m",
                    colors.get_color(&record.level()).to_fg_str()),
                date = chrono::Utc::now().format("[%+]"), // %+ = RFC 3339 date & time format
                file = file,
                line = record.line().unwrap_or(0),
                target = module_or_target,
                level = record.level(),
                message = message
            ))
        })
        .chain(fern::log_file(log_filepath)?);

    let stdout_config = fern::Dispatch::new()
        .level(stdout_level)
        .format(move |out, message, record| {
            let module_or_target = record.module_path().unwrap_or(record.target());
            let mut show_code_location = false;

            if module_or_target == "dv_client" {
                show_code_location = true;
            } else if module_or_target == "dv_full_node" {
                show_code_location = true;
            } else if module_or_target.starts_with(DATAVIR_PKG_PREIX) {
                show_code_location = true;
            }
            
            let code_location = match show_code_location {
                true => {
                    format_args!("{file}:{line: <4}",
                        file = record.file().unwrap_or("?"),
                        line = record.line().unwrap_or(0)).to_string()},
                false => {
                    format_args!("{target}",
                        target = module_or_target).to_string()}
            };

            out.finish(format_args!(
                "{color_code}[{date}][{level: <5}][{code_location}] {message}\x1B[0m",
                color_code = format_args!(
                    "\x1B[{}m",
                    colors.get_color(&record.level()).to_fg_str()),
                date = chrono::Local::now().format("%H:%M:%S"),
                level = record.level(),
                code_location = code_location,
                message = message
            ))
        })
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}
