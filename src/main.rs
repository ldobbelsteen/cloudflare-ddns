mod cloudflare;
mod ip;

use clap::Parser;
use cloudflare::{build_client, dynamic_dns_routine, get_zone_id};
use serde::Deserialize;

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
async fn main() {
    let args = Args::parse();

    let config_file = std::fs::File::open(&args.config).unwrap_or_else(|err| {
        eprintln!(
            "[FATAL] failed to read config file at '{}': {}",
            args.config, err
        );
        std::process::exit(1);
    });

    let mut config: Config = serde_yaml::from_reader(config_file).unwrap_or_else(|err| {
        eprintln!(
            "[FATAL] failed to parse config file at '{}': {}",
            args.config, err
        );
        std::process::exit(1);
    });

    config.record_name = if config.record_name == "@" {
        config.zone_name.clone()
    } else {
        config.record_name + "." + &config.zone_name
    };

    let client = build_client(&config.api_token).await.unwrap_or_else(|err| {
        eprintln!("[FATAL] failed to build api client: {}", err);
        std::process::exit(1);
    });

    let zone = get_zone_id(&client, &config.zone_name)
        .await
        .unwrap_or_else(|err| {
            eprintln!("[FATAL] failed to find specified zone: {}", err);
            std::process::exit(1);
        });

    if config.interval == 0 {
        dynamic_dns_routine(&config, &client, &zone)
            .await
            .unwrap_or_else(|err| {
                eprintln!("[FATAL] update failed: {}", err);
                std::process::exit(1);
            });
    } else {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(config.interval));
        loop {
            dynamic_dns_routine(&config, &client, &zone)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("[FATAL] routine update failed: {}", err);
                    std::process::exit(1);
                });
            interval.tick().await;
        }
    };
}
