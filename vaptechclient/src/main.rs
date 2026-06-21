mod app;
mod config;
mod hmi;
mod moonraker;
mod ui;

use anyhow::Result;
use clap::Parser;
use config::Config;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "config/config.example.toml")]
    config: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = Config::load(args.config)?;

    println!("{:#?}", cfg);
    println!("Moonraker WS: {}", cfg.moonraker_ws_url());
    println!("Moonraker HTTP: {}", cfg.moonraker_http_url());

    Ok(())
}
