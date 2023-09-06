use error_chain::error_chain;
use std::net::{Ipv4Addr, Ipv6Addr};

error_chain! {
    foreign_links {
        Reqwest(reqwest::Error);
        Parse(std::net::AddrParseError);
    }
}

pub async fn get_public_ipv4() -> Result<Option<Ipv4Addr>> {
    match reqwest::get("https://ipv4.icanhazip.com").await {
        Ok(resp) => {
            let text = resp.text().await?;
            let ip = text.trim().parse()?;
            Ok(Some(ip))
        }
        Err(e) => {
            let err_str = e.to_string();
            if !err_str.contains("unreachable") {
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
            let err_str = e.to_string();
            if !err_str.contains("unreachable") {
                Err(e.into())
            } else {
                Ok(None)
            }
        }
    }
}
