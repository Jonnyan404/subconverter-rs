use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlessProxy {
    pub uuid: String,
    pub flow: Option<String>,
    pub tls: bool,
    pub alpn: HashSet<String>,
    pub udp: bool,
    pub packet_addr: Option<bool>,
    pub xudp: Option<bool>,
    pub packet_encoding: Option<String>,
    pub network: Option<String>,
    pub reality_public_key: Option<String>,
    pub reality_short_id: Option<String>,
    pub http_method: Option<String>,
    pub http_path: Option<String>,
    pub http_headers: Option<HashMap<String, Vec<String>>>,
    pub h2_host: Option<Vec<String>>,
    pub h2_path: Option<String>,
    pub grpc_service_name: Option<String>,
    pub ws_path: Option<String>,
    pub ws_headers: Option<HashMap<String, String>>,
    pub skip_cert_verify: Option<bool>,
    pub fingerprint: Option<String>,
    pub servername: Option<String>,
    pub client_fingerprint: Option<String>,

    // ✅ 添加缺失的 SMUX 字段
    pub smux_enabled: Option<bool>,
    pub smux_protocol: Option<String>,
    pub smux_padding: Option<bool>,
    pub smux_max_connections: Option<String>,
    pub smux_min_streams: Option<String>,
    pub smux_statistic: Option<bool>,
    pub smux_only_tcp: Option<bool>,

    // ✅ 添加缺失的 Brutal 字段
    pub brutal_enabled: Option<bool>,
    pub brutal_up: Option<String>,
    pub brutal_down: Option<String>,
}

impl Default for VlessProxy {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            flow: None,
            tls: false,
            alpn: HashSet::new(),
            udp: true,
            packet_addr: None,
            xudp: None,
            packet_encoding: None,
            network: None,
            reality_public_key: None,
            reality_short_id: None,
            http_method: None,
            http_path: None,
            http_headers: None,
            h2_host: None,
            h2_path: None,
            grpc_service_name: None,
            ws_path: None,
            ws_headers: None,
            skip_cert_verify: None,
            fingerprint: None,
            servername: None,
            client_fingerprint: None,
            smux_enabled: None,
            smux_protocol: None,
            smux_padding: None,
            smux_max_connections: None,
            smux_min_streams: None,
            smux_statistic: None,
            smux_only_tcp: None,
            brutal_enabled: None,
            brutal_up: None,
            brutal_down: None,
        }
    }
}
