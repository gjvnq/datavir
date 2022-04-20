#[allow(unused_imports)]
use datavir::prelude::*;
use datavir::ws_server::WSServer;

async fn real_main() -> i32 {
    let args = clap::Command::new("dv-full-node")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .about("A datavir full node")
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("Increases logging verbosity each use for up to 4 times"),
        )
        .arg(
            clap::Arg::new("ADDR")
                .help("Address in which to listen for connections")
                .default_value(DEFAULT_WS_ADDR)
                .index(1),
        )
        .get_matches();

    // Setup and test logger
    let verbosity: u64 = args.occurrences_of("verbose");
    default_logging_setup(verbosity, "dv-full-node.log").expect("failed to initialize log");
    info!("DataVir Full Node v{} starting up!", DATAVIR_VERSION);
    warn!("WARN  output enabled.");
    debug!("DEBUG output enabled.");
    trace!("TRACE output enabled.");
    debug!("Arg matches: {:?}", args);

    let mut server = WSServer::new(args.value_of("ADDR").expect("missing address"));
    if let Err(_err) = server.prepare().await {
        return 1;
    }
    if let Err(_err) = server.main_loop().await {
        return 2;
    }

    0
}

#[tokio::main]
async fn main() {
    unsafe {init_uuid_context();}
    std::process::exit(real_main().await);
}