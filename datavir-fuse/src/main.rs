mod bundle;
mod bundle_file;
mod hacks;
mod inode_record;
mod node_type;
mod prelude;
mod schema;

use crate::prelude::*;
use crate::schema::open_database;
use clap::{App, Arg};
use fern::colors::{Color, ColoredLevelConfig};

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
            let mut module_or_target = record.module_path().unwrap_or(record.target());
            if module_or_target.starts_with("datavir_fuse::") {
                module_or_target = "datavir_fuse"
            }
            out.finish(format_args!(
                "{date}[{level: <5}][{target}][{file}:{line: <4}] {message}",
                date = chrono::Utc::now().format("[%Y-%m-%dT%H:%M:%SZ]"),
                file = record.file().unwrap_or("?"),
                line = record.line().unwrap_or(0),
                target = module_or_target,
                level = colors.color(record.level()),
                message = message
            ))
        })
        .chain(fern::log_file("datavir.log")?);

    let stdout_config = fern::Dispatch::new()
        .level(stdout_level)
        .format(move |out, message, record| {
            let mut module_or_target = record.module_path().unwrap_or(record.target());
            if module_or_target.starts_with("datavir_fuse::") {
                module_or_target = "datavir_fuse"
            }
            if module_or_target == "datavir_fuse" {
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
                    target = module_or_target,
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
    trace!(
        "+{}:(what={:?}, path={:?})",
        stringify!(ensure_dir_exists),
        what,
        path
    );
    if !path.exists() {
        debug!(
            "{} does not exist, will try to make it if parent exists",
            what
        );
        let parent = path.parent();
        debug!("{}.parent() = {:?}", what, parent);
        if let Some(parent) = parent {
            if parent.exists() {
                match fs::create_dir(path) {
                    Ok(_) => {
                        debug!("Created {}", what);
                        trace!(
                            "-{}:(what={:?}, path={:?}) -> {:?}",
                            stringify!(ensure_dir_exists),
                            what,
                            path,
                            true
                        );
                        return true;
                    }
                    Err(err) => {
                        error!("Failed to make {}: {:?}", what, err);
                        trace!(
                            "-{}:(what={:?}, path={:?}) -> {:?}",
                            stringify!(ensure_dir_exists),
                            what,
                            path,
                            false
                        );
                        return false;
                    }
                }
            } else {
                debug!("{}'s parent does not exist", what);
                error!("{} {:?} does not exists", what, path);
                trace!(
                    "-{}:(what={:?}, path={:?}) -> {:?}",
                    stringify!(ensure_dir_exists),
                    what,
                    path,
                    false
                );
                return false;
            }
        } else {
            error!("Failed to get {}'s parent", what);
            error!("{} {:?} does not exists", what, path);
            trace!(
                "-{}:(what={:?}, path={:?}) -> {:?}",
                stringify!(ensure_dir_exists),
                what,
                path,
                false
            );
            return false;
        }
    } else if !path.is_dir() {
        error!("{:?} is not a directory", path);
        trace!(
            "-{}:(what={:?}, path={:?}) -> {:?}",
            stringify!(ensure_dir_exists),
            what,
            path,
            false
        );
        return false;
    } else {
        debug!("Great! {} exists and is a folder", what);
        trace!(
            "-{}:(what={:?}, path={:?}) -> {:?}",
            stringify!(ensure_dir_exists),
            what,
            path,
            true
        );
        return true;
    }
}

fn main() {
    let cmd_arguments = App::new("datavir")
        .author("Gabriel Queiroz <gabrieljvnq@gmail.com>")
        .about("A document organizer that supports rich metadata, filters and subfiles")
        .arg(
            Arg::with_name("DATA_DIR")
                .help("Sets the the directory that will hold the actual data")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("MOUNT_POINT")
                .help("Sets the the mount point for the file system")
                .required(true)
                .index(2),
        )
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

    // TODO: move this code to somewhere more appropriate

    // Ensure DATA_DIR exists and is a folder
    let data_dir: &str = cmd_arguments.value_of("DATA_DIR").unwrap();
    debug!("DATA_DIR = {:?}", data_dir);
    let data_dir = Path::new(data_dir);
    if !ensure_dir_exists("DATA_DIR", data_dir) {
        return;
    }

    // Ensure MOUNT_POINT exists and is a folder
    let mount_dir: &str = cmd_arguments.value_of("MOUNT_POINT").unwrap();
    debug!("MOUNT_POINT = {:?}", mount_dir);
    let mount_dir = Path::new(mount_dir);
    if !ensure_dir_exists("MOUNT_POINT", mount_dir) {
        return;
    }

    // FUSE doesn't like when the mount point is not empty
    let mount_dir_listing = mount_dir.read_dir();
    if let Ok(mut dir_listing) = mount_dir_listing {
        let is_empty = dir_listing.next().is_none();
        if !is_empty {
            error!("Mount point {:?} is not is_empty", mount_dir);
            return;
        }
    } else {
        error!("Failed to get contents of {:?}", mount_dir);
    }

    // Open database
    let mut db_path = data_dir.to_path_buf();
    db_path.push("datavir.sqlite");
    let conn = match open_database(db_path.as_path()) {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to open database at {:?}: {:?}", db_path, err);
            return;
        }
    };

    info!("Database ready!");

    // Reserve some inodes if not already

    // Get inode counter

    // Mount FS

    match conn.close() {
        Err(err) => {
            error!("Failed to close database: {:?}", err);
        }
        _ => {}
    }
    info!("Database closed");
}
