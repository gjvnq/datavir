#[allow(unused_imports)]
use datavir::prelude::*;
use datavir::ws_client::WSClient;
use std::thread;

async fn real_main() -> i32 {
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
                .default_value(DEFAULT_WS_ADDR_URL)
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
    debug!("Arg matches: {:?}", args);

    // TODO: run multiple parallel requests

    let mut client = match WSClient::new(args.value_of("ADDR").expect("missing address")).await {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to start WSClient: {:?}", err);
            return 1;
        }
    };


    // futures::spawn(client.main_loop().await);
    thread::spawn(async {
        client.main_loop().await;
    });

    {
        let time1 = client.ask_time();
        let time2 = client.ask_time();
        let time3 = client.ask_time();
        info!("Got time: {:?}", time3.await);
        info!("Got time: {:?}", time1.await);
        info!("Got time: {:?}", time2.await);
    }

    0
}

#[tokio::main]
async fn main() {
    unsafe {init_uuid_context();}
    std::process::exit(real_main().await);
}