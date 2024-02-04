#![warn(clippy::pedantic)]

use anyhow::Result;
use clap::Parser;
use cloudflare::{build_client, get_zone_id, verify_token};
use ddns::routine;
use serde::Deserialize;
use std::{fs::File, time::Duration};

mod cloudflare;
mod ddns;
mod ip;

#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
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

#[derive(Debug, Parser)]
struct Args {
    #[clap(index = 1, default_value = "./config.yml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder().init();

    let args = Args::parse();
    let file = File::open(&args.config)?;
    let mut config: Config = serde_yaml::from_reader(file)?;

    config.record_name = if config.record_name == "@" {
        config.zone_name.clone()
    } else {
        config.record_name + "." + &config.zone_name
    };

    let client = build_client(&config.api_token).await?;
    verify_token(&client).await?;

    let zone = get_zone_id(&client, &config.zone_name).await?;

    if config.interval == 0 {
        routine(&config, &client, &zone).await;
    } else {
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval));
        loop {
            interval.tick().await;
            routine(&config, &client, &zone).await;
        }
    };

    Ok(())
}
