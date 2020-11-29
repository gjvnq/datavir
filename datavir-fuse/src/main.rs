mod bundle;
mod bundle_file;
mod hacks;
mod inode_record;
mod node_type;

use fern::colors::{Color, ColoredLevelConfig};

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
                    "[{date}][{level: <5}][{file}:{line}] {message}",
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

fn main() {
    let cmd_arguments = clap::App::new("datavir")
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Increases logging verbosity each use for up to 4 times"),
        )
        .get_matches();
    let verbosity: u64 = cmd_arguments.occurrences_of("verbose");
    setup_logging(verbosity).expect("failed to initialize log");

    info!("DataVir v0.0.1 starting up!");
    warn!("WARN  output enabled.");
    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");
}
