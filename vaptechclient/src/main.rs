use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;
use vaptechclient::app::event::AppEvent;
use vaptechclient::app::runner::AppRunner;
use vaptechclient::config::Config;
use vaptechclient::hmi::event::HmiEvent;
use vaptechclient::hmi::serial_service::HmiSerialService;
use vaptechclient::moonraker::client::MoonrakerClient;
use vaptechclient::runtime::Runtime;
use vaptechclient::thumbnail::resolver::ThumbnailResolverConfig;
use vaptechclient::thumbnail::worker::ThumbnailWorker;

// Точка входа пока намеренно тонкая: поднимаем внешние сервисы и
// связываем их через очереди событий/команд. Бизнес-логика живет в AppRunner.
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
    let (hmi_event_tx, mut hmi_event_rx) = mpsc::channel(config.tx.queue_size);
    let (thumbnail_request_tx, thumbnail_request_rx) = mpsc::channel(config.tx.queue_size);
    let (thumbnail_result_tx, thumbnail_result_rx) = mpsc::channel(config.tx.queue_size);
    let (moonraker_request_tx, moonraker_request_rx) = mpsc::channel(config.tx.queue_size);

    let app_event_tx_from_hmi = app_event_tx.clone();
    let touch_log_level = config.log.touch_level.clone();
    let numeric_log_level = config.log.numeric_level.clone();

    tokio::spawn(async move {
        while let Some(hmi_event) = hmi_event_rx.recv().await {
            log_hmi_touch_event(&hmi_event, &touch_log_level);
            log_hmi_numeric_event(&hmi_event, &numeric_log_level);

            if app_event_tx_from_hmi
                .send(AppEvent::hmi(hmi_event))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // HMI service единственный пишет в UART дисплея. Остальной код отправляет
    // только HmiCommand в очередь, чтобы не блокировать websocket и reducer.
    let hmi_service = HmiSerialService::open(
        config.hmi.serial.clone(),
        config.hmi.baud,
        hmi_command_rx,
        hmi_event_tx,
    )?;

    tokio::spawn(async move {
        if let Err(error) = hmi_service.run().await {
            tracing::error!(?error, "HMI serial service stopped");
        }
    });

    let thumbnail_worker = ThumbnailWorker::new(thumbnail_request_rx, thumbnail_result_tx)
        .with_resolver_config(ThumbnailResolverConfig::new(
            config.moonraker_http_url(),
            config.ui.thumbnail_cache.clone(),
        ));

    tokio::spawn(async move {
        if let Err(error) = thumbnail_worker.run().await {
            tracing::error!(?error, "thumbnail worker stopped");
        }
    });

    // Moonraker client подписывается на статусы и принимает ограниченный
    // набор управляющих запросов. Сейчас write-path включен только для
    // caselight, остальные MoonrakerRequest runtime не пересылает.
    let moonraker_client = MoonrakerClient::new(config.moonraker_ws_url(), app_event_tx.clone())
        .with_request_rx(moonraker_request_rx);

    tokio::spawn(async move {
        if let Err(error) = moonraker_client.run().await {
            tracing::error!(?error, "Moonraker client stopped");
        }
    });

    Runtime::with_thumbnails(
        AppRunner::new(),
        app_event_rx,
        hmi_command_tx,
        thumbnail_request_tx,
        thumbnail_result_rx,
    )
    .with_moonraker_requests(moonraker_request_tx)
    .run()
    .await?;

    Ok(())
}

fn log_hmi_touch_event(event: &HmiEvent, level: &str) {
    let HmiEvent::Touch { page, component } = event else {
        return;
    };

    match level.to_ascii_lowercase().as_str() {
        "off" | "none" | "disable" | "disabled" => {}
        "trace" => tracing::trace!(page = *page, component = *component, "HmiEvent touch"),
        "debug" => tracing::debug!(page = *page, component = *component, "HmiEvent touch"),
        "info" => tracing::info!(page = *page, component = *component, "HmiEvent touch"),
        "warn" => tracing::warn!(page = *page, component = *component, "HmiEvent touch"),
        "error" => tracing::error!(page = *page, component = *component, "HmiEvent touch"),
        unknown => tracing::warn!(
            configured_level = unknown,
            page = *page,
            component = *component,
            "unknown touch log level; falling back to debug"
        ),
    }
}

fn log_hmi_numeric_event(event: &HmiEvent, level: &str) {
    let HmiEvent::NumericInput {
        page,
        component,
        value,
    } = event
    else {
        return;
    };

    match level.to_ascii_lowercase().as_str() {
        "off" | "none" | "disable" | "disabled" => {}
        "trace" => {
            tracing::trace!(
                page = *page,
                component = *component,
                value = *value,
                "HmiEvent numeric"
            )
        }
        "debug" => {
            tracing::debug!(
                page = *page,
                component = *component,
                value = *value,
                "HmiEvent numeric"
            )
        }
        "info" => {
            tracing::info!(
                page = *page,
                component = *component,
                value = *value,
                "HmiEvent numeric"
            )
        }
        "warn" => {
            tracing::warn!(
                page = *page,
                component = *component,
                value = *value,
                "HmiEvent numeric"
            )
        }
        "error" => {
            tracing::error!(
                page = *page,
                component = *component,
                value = *value,
                "HmiEvent numeric"
            )
        }
        unknown => tracing::warn!(
            configured_level = unknown,
            page = *page,
            component = *component,
            value = *value,
            "unknown numeric log level; falling back to debug"
        ),
    }
}
