use crate::ddns::Record;
use anyhow::{bail, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::IpAddr;

const API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

pub async fn build_client(token: &str) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))?,
    );
    let client = ClientBuilder::new().default_headers(headers).build()?;
    Ok(client)
}

pub async fn verify_token(client: &Client) -> Result<()> {
    let url = format!("{API_BASE_URL}/user/tokens/verify");
    let resp = client.get(url).send().await?;
    resp.error_for_status()?;
    Ok(())
}

pub async fn get_zone_id(client: &Client, zone_name: &str) -> Result<String> {
    #[derive(Debug, Deserialize)]
    struct ZoneResult {
        id: String,
    }

    #[derive(Debug, Deserialize)]
    struct Response {
        result: Vec<ZoneResult>,
    }

    let url = format!("{API_BASE_URL}/zones?name={zone_name}");
    let resp = client.get(url).send().await?;

    let json: Response = resp.json().await?;
    if json.result.is_empty() {
        bail!("zone '{}' not found", zone_name);
    }

    Ok(json.result[0].id.clone())
}

pub async fn get_records(
    client: &Client,
    zone_id: &str,
    record_name: &str,
) -> Result<(Option<Record>, Option<Record>)> {
    #[derive(Debug, Deserialize)]
    struct Response {
        result: Vec<Record>,
    }

    let url = format!("{API_BASE_URL}/zones/{zone_id}/dns_records?type=A,AAAA&name={record_name}");
    let resp = client.get(url).send().await?;

    let json: Response = resp.json().await?;
    let mut a_record: Option<Record> = None;
    let mut aaaa_record: Option<Record> = None;
    for record in json.result {
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

pub async fn delete_record(client: &Client, record: Record) -> Result<()> {
    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    client.delete(url).send().await?;
    Ok(())
}

pub async fn update_record(client: &Client, record: Record, new_ip: IpAddr) -> Result<Record> {
    #[derive(Debug, Deserialize)]
    struct Response {
        result: Record,
    }

    let url = format!(
        "{}{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records/", record.id
    );
    let mut data = HashMap::new();
    data.insert("content", new_ip.to_string());
    let resp = client.patch(url).json(&data).send().await?;

    let json: Response = resp.json().await?;
    Ok(json.result)
}

pub async fn create_record(client: &Client, record: Record) -> Result<Record> {
    #[derive(Debug, Deserialize)]
    struct Response {
        result: Record,
    }

    let url = format!(
        "{}{}{}{}",
        API_BASE_URL, "/zones/", record.zone_id, "/dns_records"
    );
    let resp = client.post(url).json(&record).send().await?;

    let json: Response = resp.json().await?;
    Ok(json.result)
}
