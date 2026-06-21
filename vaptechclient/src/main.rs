use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;
use vaptechclient::app::runner::AppRunner;
use vaptechclient::config::Config;
use vaptechclient::hmi::serial_service::HmiSerialService;
use vaptechclient::moonraker::client::MoonrakerClient;
use vaptechclient::runtime::Runtime;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "config/config.example.toml")]
    config: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load(args.config)?;

    let log_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.log.level.as_str()));

    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    tracing::info!(
        hmi_serial = %config.hmi.serial,
        hmi_baud = config.hmi.baud,
        moonraker = %config.moonraker_ws_url(),
        "starting vaptechclient runtime"
    );

    let (app_event_tx, app_event_rx) = mpsc::channel(config.tx.queue_size);
    let (hmi_command_tx, hmi_command_rx) = mpsc::channel(config.tx.queue_size);

    let hmi_service = HmiSerialService::open(
        config.hmi.serial.clone(),
        config.hmi.baud,
        hmi_command_rx,
        app_event_tx.clone(),
    )?;

    tokio::spawn(async move {
        if let Err(error) = hmi_service.run().await {
            tracing::error!(?error, "HMI serial service stopped");
        }
    });

    let moonraker_client = MoonrakerClient::new(config.moonraker_ws_url(), app_event_tx.clone());

    tokio::spawn(async move {
        if let Err(error) = moonraker_client.run().await {
            tracing::error!(?error, "Moonraker client stopped");
        }
    });

    Runtime::new(AppRunner::new(), app_event_rx, hmi_command_tx)
        .run()
        .await?;

    Ok(())
}
