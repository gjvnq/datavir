#[path="../prelude.rs"]
mod prelude;

#[allow(unused_imports)]
use crate::prelude::*;

fn real_main() -> i32 {
    let args = clap::Command::new("dv-client")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .about("Console for connecting to a datavir full node")
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("Increases logging verbosity each use for up to 4 times"),
        )
        .arg(
            clap::Arg::new("ADDR")
                .help("Address of the datavir full node")
                .required(true)
                .index(1),
        )
        .get_matches();

    // Setup and test logger
    let verbosity: u64 = args.occurrences_of("verbose");
    default_logging_setup(verbosity, "dv-client.log").expect("failed to initialize log");
    info!("DataVir Client v{} starting up!", DATAVIR_VERSION);
    warn!("WARN  output enabled.");
    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");

    println!("{:?}", args);
    0
}

fn main() {
    unsafe {init_uuid_context();}
    std::process::exit(real_main());
}