use crate::ip::{get_public_ipv4, get_public_ipv6};
use crate::Config;
use error_chain::error_chain;
use reqwest::{header, Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

const API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

error_chain! {
    foreign_links {
        Reqwest(reqwest::Error);
        Ip(crate::ip::Error);
    }

    errors {
        ZoneNotFound(zone: String) {
            display("zone '{}' not found", zone)
        }
        InvalidTokenCharacters(token: String) {
            display("token '{}' contains invalid characters", token)
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct Record {
    id: String,
    name: String,
    r#type: String,
    zone_id: String,
    content: IpAddr,
    proxied: bool,
    ttl: u64,
}

enum Action {
    Create(Record),
    Update(Record, IpAddr),
    Delete(Record),
}

pub async fn build_client(token: &str) -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    let bearer = header::HeaderValue::try_from(&format!("Bearer {}", token))
        .or_else(|_| Err(ErrorKind::InvalidTokenCharacters(token.into())))?;
    headers.insert(header::AUTHORIZATION, bearer);
    let client = ClientBuilder::new().default_headers(headers).build()?;
    let url = format!("{}/user/tokens/verify", API_BASE_URL);
    let resp = client.get(url).send().await?;
    resp.error_for_status()?;
    Ok(client)
}

pub async fn get_zone_id(client: &Client, zone_name: &str) -> Result<String> {
    let url = format!("{}/zones?name={}", API_BASE_URL, zone_name);
    let resp = client.get(url).send().await?;

    #[derive(Deserialize)]
    struct ZoneResult {
        id: String,
    }

    #[derive(Deserialize)]
    struct Response {
        result: Vec<ZoneResult>,
    }

    let json: Response = resp.json().await?;
    if json.result.len() == 0 {
        return Err(ErrorKind::ZoneNotFound(zone_name.into()).into());
    }

    Ok(json.result[0].id.to_owned())
}

pub async fn dynamic_dns_routine(config: &Config, client: &Client, zone_id: &str) -> Result<()> {
    let (existing_a_record, existing_aaaa_record) =
        get_records(client, zone_id, &config.record_name).await?;

    let ipv4 = if !config.disable_ipv4 {
        get_public_ipv4().await?
    } else {
        None
    };

    let ipv6 = if !config.disable_ipv6 {
        get_public_ipv6().await?
    } else {
        None
    };

    let mut actions: Vec<Action> = Vec::new();

    match &existing_a_record {
        Some(r) => match ipv4 {
            Some(ip) => {
                if ip != r.content {
                    actions.push(Action::Update(r.clone(), ip.into()));
                }
            }
            None => {
                if config.delete_records {
                    actions.push(Action::Delete(r.clone()));
                }
            }
        },
        None => {
            if let Some(ip) = ipv4 {
                if config.create_records {
                    actions.push(Action::Create(Record {
                        id: "".into(),
                        name: config.record_name.clone(),
                        r#type: "A".into(),
                        zone_id: zone_id.into(),
                        content: ip.into(),
                        proxied: match &existing_aaaa_record {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &existing_aaaa_record {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    }));
                }
            }
        }
    }

    match &existing_aaaa_record {
        Some(r) => match ipv6 {
            Some(ip) => {
                if ip != r.content {
                    actions.push(Action::Update(r.clone(), ip.into()));
                }
            }
            None => {
                if config.delete_records {
                    actions.push(Action::Delete(r.clone()));
                }
            }
        },
        None => {
            if let Some(ip) = ipv6 {
                if config.create_records {
                    actions.push(Action::Create(Record {
                        id: "".into(),
                        name: config.record_name.clone(),
                        r#type: "AAAA".into(),
                        zone_id: zone_id.into(),
                        content: ip.into(),
                        proxied: match &existing_a_record {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &existing_a_record {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    }));
                }
            }
        }
    }

    for action in actions.into_iter() {
        match action {
            Action::Create(r) => {
                match create_record(client, r).await {
                    Err(e) => eprintln!("[WARN] error while creating record: {}", e),
                    Ok(r) => println!("[INFO] {} record created with IP {}, a TTL of {} second(s) and proxying {}...", r.r#type, r.content, r.ttl, r.proxied)
                };
            }
            Action::Update(r, ip) => {
                match update_record(client, r, ip).await {
                    Err(e) => eprintln!("[WARN] error while updating record: {}", e),
                    Ok(r) => println!("[INFO] {} record IP updated to {}...", r.r#type, r.content),
                };
            }
            Action::Delete(r) => {
                let record_type = r.r#type.clone();
                match delete_record(client, r).await {
                    Err(e) => eprintln!("[WARN] error while deleting record: {}", e),
                    Ok(()) => println!("[INFO] {} record has been deleted...", record_type),
                };
            }
        }
    }

    Ok(())
}

async fn get_records(
    client: &Client,
    zone_id: &str,
    record_name: &str,
) -> Result<(Option<Record>, Option<Record>)> {
    let url = format!(
        "{}/zones/{}/dns_records?type=A,AAAA&name={}",
        API_BASE_URL, zone_id, record_name
    );
    let resp = client.get(url).send().await?;

    #[derive(Deserialize)]
    struct Response {
        result: Vec<Record>,
    }

    let json: Response = resp.json().await?;
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

    Ok((a_record, aaaa_record))
}

async fn delete_record(client: &Client, record: Record) -> Result<()> {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    client.delete(url).send().await?;
    Ok(())
}

async fn update_record(client: &Client, record: Record, new_ip: IpAddr) -> Result<Record> {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    let mut data = HashMap::new();
    data.insert("content", new_ip.to_string());
    let resp = client.patch(url).json(&data).send().await?;

    #[derive(Deserialize)]
    struct Response {
        result: Record,
    }

    let json: Response = resp.json().await?;
    Ok(json.result)
}

async fn create_record(client: &Client, record: Record) -> Result<Record> {
    let url = format!(
        "{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records"
    );
    let resp = client.post(url).json(&record).send().await?;

    #[derive(Deserialize)]
    struct Response {
        result: Record,
    }

    let json: Response = resp.json().await?;
    Ok(json.result)
}
