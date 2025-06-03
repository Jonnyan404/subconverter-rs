use crate::{models::HYSTERIA_DEFAULT_GROUP, utils::url_decode, Proxy};
use std::collections::HashMap;
use url::Url;

/// Parse speed string and extract numeric value
/// Examples: "11 Mbps" -> 11, "55Mbps" -> 55, "100" -> 100
fn parse_speed(speed_str: &str) -> Option<u32> {
    let cleaned = speed_str
        .to_lowercase()
        .replace("mbps", "")
        .replace(" ", "")
        .trim()
        .to_string();

    cleaned.parse::<u32>().ok()
}

/// Parse a Hysteria link into a Proxy object
pub fn explode_hysteria(hysteria: &str, node: &mut Proxy) -> bool {
    // Check if the link starts with hysteria://
    if !hysteria.starts_with("hysteria://") {
        return false;
    }

    // Try to parse as URL
    let url = match Url::parse(hysteria) {
        Ok(url) => url,
        Err(_) => return false,
    };

    // Extract host and port
    let host = match url.host_str() {
        Some(host) => host,
        None => return false,
    };
    let port = url.port().unwrap_or(443);

    // Extract parameters from the query string
    let mut params = HashMap::new();
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), url_decode(&value));
    }

    // Extract auth string
    let auth = params.get("auth").map(|s| s.as_str()).unwrap_or("");

    // Extract protocol
    let protocol = params.get("protocol").map(|s| s.as_str()).unwrap_or("udp");

    // Extract up and down speeds - 支持多种格式
    let up_speed = if let Some(up_str) = params.get("up") {
        parse_speed(up_str)
    } else if let Some(up_str) = params.get("upmbps") {
        up_str.parse::<u32>().ok()
    } else {
        None
    };

    let down_speed = if let Some(down_str) = params.get("down") {
        parse_speed(down_str)
    } else if let Some(down_str) = params.get("downmbps") {
        down_str.parse::<u32>().ok()
    } else {
        None
    };

    // Extract ALPN
    let alpn_str = params.get("alpn").map(|s| s.as_str()).unwrap_or("");
    let mut alpn = Vec::new();
    if !alpn_str.is_empty() {
        for a in alpn_str.split(',') {
            alpn.push(a.trim().to_string());
        }
    }

    // Extract obfs
    let obfs = params.get("obfs").map(|s| s.as_str()).unwrap_or("");
    let obfs_param = params.get("obfsParam").map(|s| s.as_str()).unwrap_or("");

    // Extract SNI
    let sni = params.get("peer").map(|s| s.as_str()).unwrap_or(host);

    // Extract insecure
    let insecure = params
        .get("insecure")
        .map(|s| s == "1" || s.to_lowercase() == "true")
        .unwrap_or(false);

    // Extract remark from the fragment
    let remark = url.fragment().unwrap_or("");
    let formatted_remark = if remark.is_empty() {
        format!("{} ({})", host, port)
    } else {
        remark.to_string()
    };

    // Create the proxy object using the hysteria_construct method
    *node = Proxy::hysteria_construct(
        HYSTERIA_DEFAULT_GROUP.to_string(),
        formatted_remark,
        host.to_string(),
        port,
        "".to_string(), // ports
        protocol.to_string(),
        obfs_param.to_string(),
        up_speed,
        down_speed,
        auth.to_string(),
        obfs.to_string(),
        sni.to_string(),
        "".to_string(), // fingerprint
        "".to_string(), // ca
        "".to_string(), // ca_str
        None,           // recv_window_conn
        None,           // recv_window
        None,           // disable_mtu_discovery
        None,           // hop_interval
        alpn,
        None, // tcp_fast_open
        Some(insecure),
        None, // underlying_proxy
    );

    true
}
