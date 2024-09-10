use crate::{
    cloudflare::{create_record, delete_record, get_records, update_record},
    ip::{get_public_ipv4, get_public_ipv6},
    Config,
};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub zone_id: String,
    pub content: IpAddr,
    pub proxied: bool,
    pub ttl: u64,
}

#[derive(Debug)]
enum Action {
    Create(Record),
    Update(Record, IpAddr),
    Delete(Record),
}

pub async fn routine(config: &Config, client: &Client, zone_id: &str) -> Result<()> {
    log::info!("running update routine...");

    let mut action_executed = false;

    if let Some(action) = get_a_record_action(config, zone_id, client).await? {
        execute_action(client, action).await?;
        action_executed = true;
    }

    if let Some(action) = get_aaaa_record_action(config, zone_id, client).await? {
        execute_action(client, action).await?;
        action_executed = true;
    }

    if !action_executed {
        log::info!("no action required...");
    }

    Ok(())
}

async fn get_a_record_action(
    config: &Config,
    zone_id: &str,
    client: &Client,
) -> Result<Option<Action>> {
    let (a_rec, aaaa_rec) = get_records(client, zone_id, &config.record_name).await?;

    match &a_rec {
        Some(r) => {
            if let Some(ipv4) = get_public_ipv4().await? {
                if ipv4 == r.content {
                    log::debug!("public ipv4 found ({}) which matches the A record...", ipv4);
                    Ok(None)
                } else {
                    Ok(Some(Action::Update(r.clone(), ipv4.into())))
                }
            } else if config.manage_records {
                Ok(Some(Action::Delete(r.clone())))
            } else {
                log::warn!(
                    "public ipv4 not found but an A record ({}) exists, consider enabling record management",
                    r.content
                );
                Ok(None)
            }
        }
        None => {
            if let Some(ipv4) = get_public_ipv4().await? {
                if config.manage_records {
                    Ok(Some(Action::Create(Record {
                        id: String::new(),
                        name: config.record_name.clone(),
                        r#type: "A".into(),
                        zone_id: zone_id.into(),
                        content: ipv4.into(),
                        proxied: match &aaaa_rec {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &aaaa_rec {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    })))
                } else {
                    log::warn!(
                        "public ipv4 found ({}) but no A record exists, consider enabling record management", ipv4
                    );
                    Ok(None)
                }
            } else {
                log::debug!("public ipv4 not found, matching the absence of an A record...");
                Ok(None)
            }
        }
    }
}

async fn get_aaaa_record_action(
    config: &Config,
    zone_id: &str,
    client: &Client,
) -> Result<Option<Action>> {
    let (a_rec, aaaa_rec) = get_records(client, zone_id, &config.record_name).await?;

    match &aaaa_rec {
        Some(r) => {
            if let Some(ipv6) = get_public_ipv6().await? {
                if ipv6 == r.content {
                    log::debug!(
                        "public ipv6 found ({}) which matches the AAAA record...",
                        ipv6
                    );
                    Ok(None)
                } else {
                    Ok(Some(Action::Update(r.clone(), ipv6.into())))
                }
            } else if config.manage_records {
                Ok(Some(Action::Delete(r.clone())))
            } else {
                log::warn!(
                    "public ipv6 not found but an AAAA record ({}) exists, consider enabling record management",
                    r.content
                );
                Ok(None)
            }
        }
        None => {
            if let Some(ipv6) = get_public_ipv6().await? {
                if config.manage_records {
                    Ok(Some(Action::Create(Record {
                        id: String::new(),
                        name: config.record_name.clone(),
                        r#type: "AAAA".into(),
                        zone_id: zone_id.into(),
                        content: ipv6.into(),
                        proxied: match &a_rec {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &a_rec {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    })))
                } else {
                    log::warn!(
                        "public ipv6 found ({}) but no AAAA record exists, consider enabling record management",
                        ipv6
                    );
                    Ok(None)
                }
            } else {
                log::debug!("public ipv6 not found, matching the absence of an AAAA record...");
                Ok(None)
            }
        }
    }
}

async fn execute_action(client: &Client, action: Action) -> Result<()> {
    match action {
        Action::Create(r) => {
            let r = create_record(client, r).await?;
            log::info!(
                "{} record created with IP {}, a TTL of {} second(s) and proxying {}...",
                r.r#type,
                r.content,
                r.ttl,
                r.proxied
            );
        }
        Action::Update(r, ip) => {
            let r = update_record(client, r, ip).await?;
            log::info!("{} record IP updated to {}...", r.r#type, r.content);
        }
        Action::Delete(r) => {
            let rtype = r.r#type.clone();
            delete_record(client, r).await?;
            log::info!("{} record has been deleted...", rtype);
        }
    }

    Ok(())
}
