#![warn(clippy::pedantic)]

use anyhow::Result;
use clap::Parser;
use cloudflare::{build_client, dynamic_dns_routine, get_zone_id};
use serde::Deserialize;
use std::{fs::File, time::Duration};

mod cloudflare;
mod ip;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    interval: u64,
    zone_name: String,
    api_token: String,
    record_name: String,
    create_records: bool,
    delete_records: bool,
    disable_ipv4: bool,
    disable_ipv6: bool,
}

#[derive(Parser)]
struct Args {
    #[clap(index = 1, default_value = "./config.yml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::builder().format_target(false).try_init()?;

    let args = Args::parse();
    let config_file = File::open(&args.config)?;
    let mut config: Config = serde_yaml::from_reader(config_file)?;

    config.record_name = if config.record_name == "@" {
        config.zone_name.clone()
    } else {
        config.record_name + "." + &config.zone_name
    };

    let client = build_client(&config.api_token).await?;
    let zone = get_zone_id(&client, &config.zone_name).await?;

    if config.interval == 0 {
        dynamic_dns_routine(&config, &client, &zone).await?;
    } else {
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval));
        interval.tick().await; // the first tick completes immediately
        loop {
            dynamic_dns_routine(&config, &client, &zone).await?;
            interval.tick().await;
        }
    };

    Ok(())
}
