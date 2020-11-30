mod bundle;
mod bundle_file;
mod hacks;
mod inode_record;
mod node_type;

use std::fs;
use std::path::Path;
use clap::{Arg, App};
use fern::colors::{Color, ColoredLevelConfig};
use  rusqlite::config::DbConfig;

#[allow(unused_imports)]
use log::{debug, info, trace, warn, error};

fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
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
            out.finish(format_args!(
                "{date}[{level: <5}][{target}][{file}:{line}] {message}",
                date = chrono::Utc::now().format("[%Y-%m-%dT%H:%M:%SZ]"),
                file = record.file().unwrap_or("?"),
                line = record.line().unwrap_or(0),
                target = record.target(),
                level = colors.color(record.level()),
                message = message
            ))
        })
        .chain(fern::log_file("datavir.log")?);

    let stdout_config = fern::Dispatch::new()
        .level(stdout_level)
        .format(move |out, message, record| {
            if record.target() == "datavir_fuse" {
                out.finish(format_args!(
                    "[{date}][{level: <5}][{file}:{line: >4}] {message}",
                    date = chrono::Local::now().format("%H:%M"),
                    level = colors.color(record.level()),
                    file = record.file().unwrap_or("?"),
                    line = record.line().unwrap_or(0),
                    message = message
                ))
            } else {
                out.finish(format_args!(
                    "[{date}][{level: <5}][{target}] {message}",
                    date = chrono::Local::now().format("%H:%M"),
                    target = record.target(),
                    level = colors.color(record.level()),
                    message = message
                ))
            }
        })
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}

fn ensure_dir_exists(what: &str, path: &Path) -> bool {
    trace!("+{}:(what={:?}, path={:?})", stringify!(ensure_dir_exists), what, path);
    if !path.exists() {
        debug!("{} does not exist, will try to make it if parent exists", what);
        let parent = path.parent();
        debug!("{}.parent() = {:?}", what, parent);
        if let Some(parent) = parent {
            if parent.exists() {
                match fs::create_dir(path) {
                    Ok(_) => {
                        debug!("Created {}", what);
                        trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, true);
                        return true
                    },
                    Err(err) => {
                        error!("Failed to make {}: {:?}", what, err);
                        trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, false);
                        return false
                    }
                }
            } else {
                debug!("{}'s parent does not exist", what);
                error!("{} {:?} does not exists", what, path);
                trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, false);
                return false
            }
        } else {
            error!("Failed to get {}'s parent", what);
            error!("{} {:?} does not exists", what, path);
            trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, false);
            return false
        }
    } else if !path.is_dir() {
        error!("{:?} is not a directory", path);
        trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, false);
        return false
    } else {
        debug!("Great! {} exists and is a folder", what);
        trace!("-{}:(what={:?}, path={:?}) -> {:?}", stringify!(ensure_dir_exists), what, path, true);
        return true
    }
}

fn main() {
    let cmd_arguments = App::new("datavir")
        .author("Gabriel Queiroz <gabrieljvnq@gmail.com>")
        .about("A document organizer that supports rich metadata, filters and subfiles")
        .arg(Arg::with_name("DATA_DIR")
            .help("Sets the the directory that will hold the actual data")
            .required(true)
            .index(1))
        .arg(Arg::with_name("MOUNT_POINT")
            .help("Sets the the mount point for the file system")
            .required(true)
            .index(2))
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Increases logging verbosity each use for up to 4 times"),
        )
        .get_matches();

    // Setup and test logger
    let verbosity: u64 = cmd_arguments.occurrences_of("verbose");
    setup_logging(verbosity).expect("failed to initialize log");
    info!("DataVir v0.0.1 starting up!");
    warn!("WARN  output enabled.");
    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");

    // Ensure DATA_DIR exists and is a folder
    let data_dir: &str = cmd_arguments.value_of("DATA_DIR").unwrap();
    debug!("DATA_DIR = {:?}", data_dir);
    let data_dir = Path::new(data_dir);
    if !ensure_dir_exists("DATA_DIR", data_dir) {
        return
    }

    // Ensure MOUNT_POINT exists and is a folder
    let mount_dir: &str = cmd_arguments.value_of("MOUNT_POINT").unwrap();
    debug!("MOUNT_POINT = {:?}", mount_dir);
    let mount_dir = Path::new(mount_dir);
    if !ensure_dir_exists("MOUNT_POINT", mount_dir) {
        return
    }

    // FUSE doesn't like when the mount point is not empty
    let mount_dir_listing = mount_dir.read_dir();
    if let Ok(mut dir_listing) = mount_dir_listing {
        let is_empty = dir_listing.next().is_none();
        if !is_empty {
            error!("Mount point {:?} is not is_empty", mount_dir);
            return
        }
    } else {
        error!("Failed to get contents of {:?}", mount_dir);
    }

    // Open database
    let mut db_path = data_dir.to_path_buf();
    db_path.push("datavir.sqlite");
    let db_conn = match rusqlite::Connection::open(db_path.as_path()) {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to open database at {:?}: {:?}", db_path, err);
            return
        }
    };
    info!("Opened database");
    // We don't need foreign keys
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, false) {
        error!("Failed to set SQLITE_DBCONFIG_ENABLE_FKEY: {:?}", err);
        return
    }
    // We do need triggers
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, true) {
        error!("Failed to set SQLITE_DBCONFIG_ENABLE_TRIGGER: {:?}", err);
        return
    }
    // We don't use full text search
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER, false) {
        error!("Failed to set SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER: {:?}", err);
        return
    }
    // Enable checkpoints (yes, it is `false` to enable)
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, false) {
        error!("Failed to set SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE: {:?}", err);
        return
    }
    // Enable "stable" query times
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_QPSG, true) {
        error!("Failed to set SQLITE_DBCONFIG_ENABLE_QPSG: {:?}", err);
        return
    }
    // Add some protection against mistakes
    if let Err(err) = db_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true) {
        error!("Failed to set SQLITE_DBCONFIG_DEFENSIVE: {:?}", err);
        return
    }
    info!("Finished setting database params");
    
}
