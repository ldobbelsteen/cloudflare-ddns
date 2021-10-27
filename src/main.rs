use log::{info, warn};
use reqwest::{
    blocking::Client,
    blocking::ClientBuilder,
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::net::IpAddr;
use std::process;
use std::thread::sleep;
use std::time::Duration;

const API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

#[derive(Debug, Deserialize)]
struct Config {
    interval: u64,
    zone_name: String,
    api_token: String,
    record_name: String,
    create_records: bool,
    delete_records: bool,
    disable_ipv4: bool,
    disable_ipv6: bool,
}

#[derive(Debug, Deserialize)]
struct Zone {
    id: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Record {
    id: String,
    name: String,
    r#type: String,
    zone_id: String,
    content: IpAddr,
    proxied: bool,
    ttl: u64,
}

#[derive(Debug, Deserialize)]
struct ZoneResults {
    result: Vec<Zone>,
}

#[derive(Debug, Deserialize)]
struct RecordResult {
    result: Record,
}

#[derive(Debug, Deserialize)]
struct RecordResults {
    result: Vec<Record>,
}

enum Action {
    Create(Record),
    Update(Record, IpAddr),
    Delete(Record),
    Nothing,
}

fn build_client(token: &str) -> Client {
    let mut headers = HeaderMap::new();
    let bearer = HeaderValue::from_str(&format!("Bearer {}", token)).unwrap_or_else(|error| {
        warn!("API token contains invalid characters: {}", error);
        process::exit(1);
    });
    headers.insert(AUTHORIZATION, bearer);
    let client = ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|error| {
            warn!("Failed to build API client: {}", error);
            process::exit(1);
        });
    let url = format!("{}{}", API_BASE_URL, "/user/tokens/verify");
    let resp = client.get(url).send().unwrap_or_else(|error| {
        warn!("Failed to verify API token: {}", error);
        process::exit(1);
    });
    if let Err(error) = resp.error_for_status() {
        warn!("API token is invalid: {}", error);
        process::exit(1);
    }
    client
}

fn get_zone(client: &Client, zone_name: &str) -> Zone {
    let url = format!("{}{}{}", API_BASE_URL, "/zones?name=", zone_name);
    let resp = client.get(url).send().unwrap_or_else(|error| {
        warn!("Failed to fetch zone ID: {}", error);
        process::exit(1);
    });
    let mut json: ZoneResults = resp.json().unwrap_or_else(|error| {
        warn!("Unexpected zone ID format: {}", error);
        process::exit(1);
    });
    if json.result.len() < 1 {
        warn!("No zones with specified name were found");
        process::exit(1);
    }
    json.result.remove(0)
}

fn get_records(
    client: &Client,
    zone: &Zone,
    record_name: &str,
) -> (Option<Record>, Option<Record>) {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", zone.id, "/dns_records?type=A,AAAA&name=", record_name
    );
    let resp = client.get(url).send().unwrap_or_else(|error| {
        warn!("Failed to fetch records: {}", error);
        process::exit(1);
    });
    let json: RecordResults = resp.json().unwrap_or_else(|error| {
        warn!("Unexpected records format: {}", error);
        process::exit(1);
    });
    let mut a_record: Option<Record> = None;
    let mut aaaa_record: Option<Record> = None;
    for record in json.result.into_iter() {
        match record.content {
            IpAddr::V4(_) => {
                a_record = Some(record);
            }
            IpAddr::V6(_) => {
                aaaa_record = Some(record);
            }
        }
    }
    (a_record, aaaa_record)
}

fn get_ip(ipv4_or_ipv6: bool, disabled: bool) -> Option<IpAddr> {
    let url = if ipv4_or_ipv6 {
        "http://ipv4.icanhazip.com"
    } else {
        "http://ipv6.icanhazip.com"
    };
    if !disabled {
        match reqwest::blocking::get(url) {
            Ok(resp) => {
                let text = resp.text().unwrap_or_else(|error| {
                    warn!("Failed to read IP: {}", error);
                    process::exit(1);
                });
                let ip = text.trim().parse().unwrap_or_else(|error| {
                    warn!("Failed to parse IP: {}", error);
                    process::exit(1);
                });
                Some(ip)
            }
            Err(error) => {
                if !error.to_string().contains("unreachable") {
                    warn!("Failed to fetch IP: {}", error);
                    process::exit(1);
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}

fn delete_record(client: &Client, record: Record) {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    client.delete(url).send().unwrap_or_else(|error| {
        warn!("Failed to delete record: {}", error);
        process::exit(1);
    });
    info!("'{}' record has been deleted...", record.r#type);
}

fn update_record(client: &Client, record: Record, new_ip: IpAddr) -> Record {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    let mut data = HashMap::new();
    data.insert("content", new_ip.to_string());
    let resp = client
        .patch(url)
        .json(&data)
        .send()
        .unwrap_or_else(|error| {
            warn!("Failed to update record: {}", error);
            process::exit(1);
        });
    let json: RecordResult = resp.json().unwrap_or_else(|error| {
        warn!("Unexpected record format: {}", error);
        process::exit(1);
    });
    info!(
        "'{}' record IP updated from {} to {}...",
        json.result.r#type, record.content, json.result.content
    );
    json.result
}

fn create_record(client: &Client, record: Record) -> Record {
    let url = format!(
        "{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records"
    );
    let resp = client
        .post(url)
        .json(&record)
        .send()
        .unwrap_or_else(|error| {
            warn!("Failed to create record: {}", error);
            process::exit(1);
        });
    let json: RecordResult = resp.json().unwrap_or_else(|error| {
        warn!("Unexpected record format: {}", error);
        process::exit(1);
    });
    info!(
        "'{}' record created with IP {}, a TTL of {} second(s) and proxying {}...",
        json.result.r#type, json.result.content, json.result.ttl, json.result.proxied
    );
    json.result
}

fn update_routine(config: &Config, client: &Client, zone: &Zone) {
    let (existing_a_record, existing_aaaa_record) = get_records(client, zone, &config.record_name);

    let a_action: Action = if let Some(record) = existing_a_record.clone() {
        if let Some(ip) = get_ip(true, config.disable_ipv4) {
            if ip != record.content {
                Action::Update(record, ip)
            } else {
                Action::Nothing
            }
        } else if config.delete_records {
            Action::Delete(record)
        } else {
            Action::Nothing
        }
    } else {
        if config.create_records {
            if let Some(ip) = get_ip(true, config.disable_ipv4) {
                Action::Create(Record {
                    content: ip,
                    id: "".to_string(),
                    r#type: "A".to_string(),
                    zone_id: zone.id.to_string(),
                    name: config.record_name.clone(),
                    ttl: existing_aaaa_record.clone().map(|r| r.ttl).unwrap_or(1),
                    proxied: existing_aaaa_record
                        .clone()
                        .map(|r| r.proxied)
                        .unwrap_or(true),
                })
            } else {
                Action::Nothing
            }
        } else {
            Action::Nothing
        }
    };

    let aaaa_action: Action = if let Some(record) = existing_aaaa_record.clone() {
        if let Some(ip) = get_ip(false, config.disable_ipv6) {
            if ip != record.content {
                Action::Update(record, ip)
            } else {
                Action::Nothing
            }
        } else if config.delete_records {
            Action::Delete(record)
        } else {
            Action::Nothing
        }
    } else {
        if config.create_records {
            if let Some(ip) = get_ip(false, config.disable_ipv6) {
                Action::Create(Record {
                    content: ip,
                    id: "".to_string(),
                    r#type: "AAAA".to_string(),
                    zone_id: zone.id.to_string(),
                    name: config.record_name.clone(),
                    ttl: existing_a_record.clone().map(|r| r.ttl).unwrap_or(1),
                    proxied: existing_a_record.clone().map(|r| r.proxied).unwrap_or(true),
                })
            } else {
                Action::Nothing
            }
        } else {
            Action::Nothing
        }
    };

    let perform_action = |action: Action| match action {
        Action::Create(rec) => {
            create_record(client, rec);
        }
        Action::Update(rec, ip) => {
            update_record(client, rec, ip);
        }
        Action::Delete(rec) => {
            delete_record(client, rec);
        }
        Action::Nothing => {}
    };
    perform_action(a_action);
    perform_action(aaaa_action);
}

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();
    info!("Starting up...");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        warn!("No config file location passed!");
        process::exit(1);
    }
    if args.len() > 2 {
        warn!("Too many arguments passed!");
        process::exit(1);
    }

    let file = File::open(&args[1]).unwrap_or_else(|error| {
        warn!("Failed to read config file: {:?}", error);
        process::exit(1);
    });

    let mut config: Config = serde_yaml::from_reader(file).unwrap_or_else(|error| {
        warn!("Invalid config file format: {:?}", error);
        process::exit(1);
    });

    config.record_name = if config.record_name == "@" {
        config.zone_name.clone()
    } else {
        config.record_name + "." + &config.zone_name
    };

    let client = build_client(&config.api_token);
    let zone = get_zone(&client, &config.zone_name);

    if config.interval == 0 {
        update_routine(&config, &client, &zone);
    } else {
        loop {
            update_routine(&config, &client, &zone);
            sleep(Duration::from_secs(config.interval));
        }
    }
}
