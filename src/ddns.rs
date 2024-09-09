use crate::{
    cloudflare::{create_record, delete_record, get_records, update_record},
    ip::{get_public_ipv4, get_public_ipv6},
    Config,
};
use log::{error, info, warn};
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

#[allow(clippy::too_many_lines)]
pub async fn routine(config: &Config, client: &Client, zone_id: &str) {
    info!("running update routine...");

    let (a_rec, aaaa_rec) = match get_records(client, zone_id, &config.record_name).await {
        Ok(res) => res,
        Err(e) => {
            error!("failed to get existing records: {}", e);
            return;
        }
    };

    let mut actions: Vec<Action> = Vec::new();

    match &a_rec {
        Some(r) => {
            let ipv4 = match get_public_ipv4().await {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get public ipv4: {}", e);
                    return;
                }
            };

            match ipv4 {
                Some(ip) => {
                    if ip != r.content {
                        actions.push(Action::Update(r.clone(), ip.into()));
                    }
                }
                None => {
                    if config.manage_records {
                        actions.push(Action::Delete(r.clone()));
                    } else {
                        warn!(
                            "public ipv4 not found but an A record ({}) exists, consider enabling record management",
                            r.content
                        );
                    }
                }
            }
        }
        None => {
            let ipv4 = match get_public_ipv4().await {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get public ipv4: {}", e);
                    return;
                }
            };

            if let Some(ip) = ipv4 {
                if config.manage_records {
                    actions.push(Action::Create(Record {
                        id: String::new(),
                        name: config.record_name.clone(),
                        r#type: "A".into(),
                        zone_id: zone_id.into(),
                        content: ip.into(),
                        proxied: match &aaaa_rec {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &aaaa_rec {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    }));
                } else {
                    warn!(
                        "public ipv4 found ({}) but no A record exists, consider enabling record management", ip
                    );
                }
            }
        }
    }

    match &aaaa_rec {
        Some(r) => {
            let ipv6 = match get_public_ipv6().await {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get public ipv6: {}", e);
                    return;
                }
            };

            match ipv6 {
                Some(ip) => {
                    if ip != r.content {
                        actions.push(Action::Update(r.clone(), ip.into()));
                    }
                }
                None => {
                    if config.manage_records {
                        actions.push(Action::Delete(r.clone()));
                    } else {
                        warn!(
                            "public ipv6 not found but an AAAA record ({}) exists, consider enabling record management",
                            r.content
                        );
                    }
                }
            }
        }
        None => {
            let ipv6 = match get_public_ipv6().await {
                Ok(res) => res,
                Err(e) => {
                    error!("failed to get public ipv6: {}", e);
                    return;
                }
            };

            if let Some(ip) = ipv6 {
                if config.manage_records {
                    actions.push(Action::Create(Record {
                        id: String::new(),
                        name: config.record_name.clone(),
                        r#type: "AAAA".into(),
                        zone_id: zone_id.into(),
                        content: ip.into(),
                        proxied: match &a_rec {
                            Some(r) => r.proxied,
                            None => true,
                        },
                        ttl: match &a_rec {
                            Some(r) => r.ttl,
                            None => 1,
                        },
                    }));
                } else {
                    warn!(
                        "public ipv6 found ({}) but no AAAA record exists, consider enabling record management",
                        ip
                    );
                }
            }
        }
    }

    if actions.is_empty() {
        info!("no action required...");
        return;
    }

    for action in actions {
        match action {
            Action::Create(r) => {
                match create_record(client, r).await {
                    Err(e) => warn!("error while creating record: {}", e),
                    Ok(r) => info!(
                        "{} record created with IP {}, a TTL of {} second(s) and proxying {}...",
                        r.r#type, r.content, r.ttl, r.proxied
                    ),
                };
            }
            Action::Update(r, ip) => {
                match update_record(client, r, ip).await {
                    Err(e) => warn!("error while updating record: {}", e),
                    Ok(r) => info!("{} record IP updated to {}...", r.r#type, r.content),
                };
            }
            Action::Delete(r) => {
                let record_type = r.r#type.clone();
                if let Err(e) = delete_record(client, r).await {
                    warn!("error while deleting record: {}", e);
                } else {
                    info!("{} record has been deleted...", record_type);
                }
            }
        }
    }
}
