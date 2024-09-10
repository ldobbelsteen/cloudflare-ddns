#![warn(clippy::pedantic)]

use anyhow::Result;
use clap::Parser;
use cloudflare::{build_client, get_zone_id, verify_token};
use ddns::routine;
use log::error;
use serde::Deserialize;
use std::{fs::read_to_string, time::Duration};

mod cloudflare;
mod ddns;
mod ip;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    zone_name: String,
    api_token: String,
    record_name: String,
    interval: u64,
    manage_records: bool,
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(index = 1, default_value = "./config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::builder().init();

    let args = Args::parse();
    let file = read_to_string(args.config)?;
    let mut config: Config = toml::from_str(&file)?;

    config.record_name = if config.record_name == "@" {
        config.zone_name.clone()
    } else {
        config.record_name + "." + &config.zone_name
    };

    let client = build_client(&config.api_token).await?;
    verify_token(&client).await?;

    let zone = get_zone_id(&client, &config.zone_name).await?;

    if config.interval == 0 {
        if let Err(e) = routine(&config, &client, &zone).await {
            error!("update routine failed: {}", e);
        }
    } else {
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval));
        interval.tick().await; // the first tick completes immediately
        loop {
            if let Err(e) = routine(&config, &client, &zone).await {
                error!("update routine failed: {}", e);
            }
            interval.tick().await;
        }
    };

    Ok(())
}
