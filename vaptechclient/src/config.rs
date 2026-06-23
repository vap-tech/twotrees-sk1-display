use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::PathBuf};

/// TOML-конфиг runtime.
///
/// RUST_LOG может переопределить log.level, а остальные поля берутся из файла.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub printer: PrinterConfig,
    pub hmi: HmiConfig,
    pub ui: UiConfig,
    pub tx: TxConfig,
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrinterConfig {
    pub host: String,
    #[serde(default = "default_moonraker_port")]
    pub moonraker_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HmiConfig {
    pub serial: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub startup_page: u16,

    #[serde(default = "default_language")]
    pub language: String,

    #[serde(default = "default_thumbnail_cache")]
    pub thumbnail_cache: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxConfig {
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    #[serde(default = "default_chunk_delay_ms")]
    pub chunk_delay_ms: u64,

    #[serde(default = "default_queue_size")]
    pub queue_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default = "default_touch_log_level")]
    pub touch_level: String,
}

impl Config {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        // Ошибки чтения/парсинга дополняем путем к файлу, чтобы на принтере было
        // понятно, какой именно config не подхватился.
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;

        let cfg: Config = toml::from_str(&raw)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;

        Ok(cfg)
    }

    pub fn moonraker_ws_url(&self) -> String {
        format!(
            "ws://{}:{}/websocket",
            self.printer.host, self.printer.moonraker_port
        )
    }

    pub fn moonraker_http_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.printer.host, self.printer.moonraker_port
        )
    }
}

fn default_moonraker_port() -> u16 {
    7125
}

fn default_baud() -> u32 {
    115200
}

fn default_language() -> String {
    "ru".to_string()
}

fn default_thumbnail_cache() -> PathBuf {
    PathBuf::from("/var/cache/vaptechclient/thumbnails")
}

fn default_chunk_size() -> usize {
    512
}

fn default_chunk_delay_ms() -> u64 {
    1
}

fn default_queue_size() -> usize {
    512
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_touch_log_level() -> String {
    "debug".to_string()
}
