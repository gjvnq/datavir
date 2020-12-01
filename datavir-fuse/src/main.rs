mod bundle;
mod bundle_file;
mod datavir_fs;
mod hacks;
mod inode_record;
mod object_type;
mod prelude;
mod schema;

use crate::datavir_fs::DataVirFS;
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

fn real_main() -> i32 {
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

    // Get data dir
    let data_dir: &str = cmd_arguments.value_of("DATA_DIR").unwrap();
    debug!("DATA_DIR = {:?}", data_dir);
    let data_dir = Path::new(data_dir);

    // Get mount point
    let mount_dir: &str = cmd_arguments.value_of("MOUNT_POINT").unwrap();
    debug!("MOUNT_POINT = {:?}", mount_dir);
    let mount_dir = Path::new(mount_dir);

    // Make FS
    let fs = match DataVirFS::new(data_dir, mount_dir) {
        Ok(v) => v,
        Err(_) => return 1,
    };

    match fs.mount() {
        Ok(_) => 0,
        Err(err) => {
            error!("Error on mount {:?}", err);
            2
        }
    }
}

fn main() {
    std::process::exit(real_main());
}
