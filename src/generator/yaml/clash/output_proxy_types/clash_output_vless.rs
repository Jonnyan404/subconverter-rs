use super::CommonProxyOptions;
use crate::models::Proxy;
use crate::utils::{is_empty_option_string, is_u32_option_zero};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reality options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RealityOptions {
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id")]
    pub short_id: String,
}

/// HTTP options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HTTPOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, Vec<String>>>,
}

/// HTTP2 options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HTTP2Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub path: Option<String>,
}

/// gRPC options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GrpcOptions {
    #[serde(
        rename = "grpc-service-name",
        skip_serializing_if = "is_empty_option_string"
    )]
    pub grpc_service_name: Option<String>,
}

/// WebSocket options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WSOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(rename = "max-early-data", skip_serializing_if = "is_u32_option_zero")]
    pub max_early_data: Option<u32>,
    #[serde(
        rename = "early-data-header-name",
        skip_serializing_if = "is_empty_option_string"
    )]
    pub early_data_header_name: Option<String>,
    #[serde(rename = "v2ray-http-upgrade", skip_serializing_if = "Option::is_none")]
    pub v2ray_http_upgrade: Option<bool>,
    #[serde(
        rename = "v2ray-http-upgrade-fast-open",
        skip_serializing_if = "Option::is_none"
    )]
    pub v2ray_http_upgrade_fast_open: Option<bool>,
}

/// SMUX options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SmuxOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<bool>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub max_connections: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub min_streams: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_tcp: Option<bool>,
}

/// Brutal options for VLESS proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrutalOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub up: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub down: Option<String>,
}

/// VLESS proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VLessProxy {
    #[serde(flatten)]
    pub common: CommonProxyOptions,
    pub uuid: String,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_addr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xudp: Option<bool>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub packet_encoding: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_opts: Option<RealityOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_opts: Option<HTTPOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h2_opts: Option<HTTP2Options>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grpc_opts: Option<GrpcOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_opts: Option<WSOptions>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub ws_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub servername: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub fingerprint: Option<String>,
    #[serde(
        skip_serializing_if = "is_empty_option_string",
        rename = "client-fingerprint"
    )]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smux: Option<SmuxOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "brutal-opts")]
    pub brutal_opts: Option<BrutalOptions>,  // ✅ 新增字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
}

impl VLessProxy {
    /// Create a new VLESS proxy
    pub fn new(common: CommonProxyOptions) -> Self {
        Self {
            common,
            uuid: String::new(),
            flow: None,
            tls: None,
            alpn: None,
            packet_addr: None,
            xudp: None,
            packet_encoding: None,
            network: None,
            reality_opts: None,
            http_opts: None,
            h2_opts: None,
            grpc_opts: None,
            ws_opts: None,
            ws_path: None,
            ws_headers: None,
            servername: None,
            fingerprint: None,
            client_fingerprint: None,
            smux: None,
            brutal_opts: None, 
            skip_cert_verify: None,
        }
    }
}

impl From<Proxy> for VLessProxy {
    fn from(proxy: Proxy) -> Self {
        let common = CommonProxyOptions::builder(
            proxy.remark.clone(), 
            proxy.hostname.clone(), 
            proxy.port
        )
        .udp(proxy.udp)
        .tfo(proxy.tcp_fast_open)
        .skip_cert_verify(proxy.allow_insecure)
        .sni(proxy.sni.clone())
        .build();

        let mut vless = VLessProxy::new(common);

        // 从 combined_proxy 获取 VLESS 特有配置或使用 Proxy 直接字段
        vless.uuid = proxy.user_id.clone().unwrap_or_default();
        vless.flow = proxy.flow.clone();
        vless.tls = Some(proxy.tls_secure);
        vless.network = proxy.transfer_protocol.clone();
        vless.packet_encoding = proxy.packet_encoding.clone();
        vless.fingerprint = proxy.fingerprint.clone();
        vless.client_fingerprint = proxy.client_fingerprint.clone();
        vless.servername = proxy.server_name.clone();

        // 处理 ALPN
        if !proxy.alpn.is_empty() {
            vless.alpn = Some(proxy.alpn.iter().cloned().collect());
        }

        // 处理 Reality 配置（需要同时有 public_key 和 short_id）
        if let (Some(public_key), Some(short_id)) = (&proxy.public_key, &proxy.reality_short_id) {
            vless.reality_opts = Some(RealityOptions {
                public_key: public_key.clone(),
                short_id: short_id.clone(),
            });
        }

        // 处理不同网络类型的特殊配置
        if let Some(network) = &proxy.transfer_protocol {
            match network.as_str() {
                "ws" => {
                    if proxy.path.is_some() || proxy.ws_headers.is_some() {
                        vless.ws_opts = Some(WSOptions {
                            path: proxy.path.clone(),
                            headers: proxy.ws_headers.clone(),
                            max_early_data: None,
                            early_data_header_name: None,
                            v2ray_http_upgrade: None,
                            v2ray_http_upgrade_fast_open: None,
                        });
                    }
                }
                "grpc" => {
                    if let Some(ref service_name) = proxy.grpc_service_name {
                        vless.grpc_opts = Some(GrpcOptions {
                            grpc_service_name: Some(service_name.clone()),
                        });
                    }
                }
                "h2" => {
                    vless.h2_opts = Some(HTTP2Options {
                        host: proxy.host.as_ref().map(|h| vec![h.clone()]),
                        path: proxy.path.clone(),
                    });
                }
                _ => {}
            }
        }

        // 仅在有显式配置时添加 SMUX
        if proxy.smux_enabled.unwrap_or(false) {
            vless.smux = Some(SmuxOptions {
                enabled: proxy.smux_enabled,
                protocol: proxy.smux_protocol.clone(),
                padding: proxy.smux_padding,
                max_connections: proxy.smux_max_connections.clone(),
                min_streams: proxy.smux_min_streams.clone(),
                statistic: proxy.smux_statistic,
                only_tcp: proxy.smux_only_tcp,
            });
        }

        // 仅在有显式配置时添加 Brutal
        if proxy.brutal_enabled.unwrap_or(false) {
            vless.brutal_opts = Some(BrutalOptions {
                enabled: proxy.brutal_enabled,
                up: proxy.brutal_up.clone(),
                down: proxy.brutal_down.clone(),
            });
        }

        vless
    }
}
