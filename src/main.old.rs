

#[macro_use]
mod prelude;
mod files;
mod schema;
mod core;
mod utils;
// mod fuse_adapter;

#[allow(unused_imports)]
use crate::prelude::*;
use clap::{app_from_crate, Arg};
#[allow(unused_imports)]
use fern::colors::{Color, ColoredLevelConfig};

use crate::core::DataVirFS;

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
                "{date}[{level: <5}][{target}][{file}:{line: <4}] {message}",
                date = chrono::Utc::now().format("[%Y-%m-%dT%H:%M:%SZ]"),
                file = file,
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
            if module_or_target.starts_with(DATAVIR_PKG_PREIX) {
                module_or_target = DATAVIR_PKG_NAME
            }
            if module_or_target == DATAVIR_PKG_NAME {
                out.finish(format_args!(
                    "[{date}][{level: <5}][{file}:{line: <4}] {message}",
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
    let cmd_arguments = app_from_crate!()
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("Increases logging verbosity each use for up to 4 times"),
        )
        .arg(
            Arg::new("DATA_DIR")
                .help("Sets the the directory that will hold the actual data")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Setup and test logger
    let verbosity: u64 = cmd_arguments.occurrences_of("verbose");
    setup_logging(verbosity).expect("failed to initialize log");
    info!("DataVir v{} starting up!", DATAVIR_VERSION);
    warn!("WARN  output enabled.");
    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");

    trace!("{}", function!());

    // Get data dir
    let data_dir: &str = cmd_arguments.value_of("DATA_DIR").unwrap();
    debug!("DATA_DIR = {:?}", data_dir);
    let data_dir = Path::new(data_dir);

    // Make FS
    let _fs = match DataVirFS::new(data_dir) {
        Ok(v) => v,
        Err(_) => return 1,
    };

    println!("hi!");
    0
}

fn init_stuff() {
    unsafe {
        init_uuid_context();
    }
}

fn main() {
    init_stuff();
    std::process::exit(real_main());
}
