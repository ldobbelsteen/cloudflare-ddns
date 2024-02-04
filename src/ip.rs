use anyhow::Result;
use std::net::{Ipv4Addr, Ipv6Addr};

pub async fn get_public_ipv4() -> Result<Option<Ipv4Addr>> {
    match reqwest::get("https://ipv4.icanhazip.com").await {
        Ok(resp) => {
            let text = resp.text().await?;
            let ip = text.trim().parse()?;
            Ok(Some(ip))
        }
        Err(e) => {
            let es = e.to_string();
            if !es.contains("tcp connect error") && !es.contains("udp connect error") {
                Err(e.into())
            } else {
                Ok(None)
            }
        }
    }
}

pub async fn get_public_ipv6() -> Result<Option<Ipv6Addr>> {
    match reqwest::get("https://ipv6.icanhazip.com").await {
        Ok(resp) => {
            let text = resp.text().await?;
            let ip = text.trim().parse()?;
            Ok(Some(ip))
        }
        Err(e) => {
            let es = e.to_string();
            if !es.contains("tcp connect error") && !es.contains("udp connect error") {
                Err(e.into())
            } else {
                Ok(None)
            }
        }
    }
}
