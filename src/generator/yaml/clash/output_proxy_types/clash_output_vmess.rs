use super::CommonProxyOptions;
use crate::models::Proxy;
use crate::utils::{is_empty_option_string, is_u32_option_zero};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vmess proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessProxy {
    #[serde(flatten)]
    pub common: CommonProxyOptions,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub uuid: Option<String>,
    /// This is required by Clash, and the name is `alterId`
    #[serde(rename = "alterId")]
    pub alter_id: u32,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub cipher: Option<String>,

    // 添加缺失的字段
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub packet_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_padding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated_length: Option<bool>,

    // TLS 相关字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub servername: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_opts: Option<VmessRealityOptions>,

    // 网络配置
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub network: Option<String>,

    // SMux 配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smux: Option<VmessSmuxOptions>,

    // 网络特定选项
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_opts: Option<VmessWsOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_opts: Option<VmessHttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h2_opts: Option<VmessH2Options>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grpc_opts: Option<VmessGrpcOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessRealityOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub short_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessSmuxOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

// 保持现有的其他选项结构体不变
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessWsOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessHttpOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessH2Options {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VmessGrpcOptions {
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub service_name: Option<String>,
}

impl VmessProxy {
    /// Create a new Vmess proxy
    pub fn new(common: CommonProxyOptions) -> Self {
        Self {
            common,
            uuid: None,
            alter_id: 0,
            cipher: None,
            packet_encoding: None,
            global_padding: None,
            authenticated_length: None,
            tls: None,
            servername: None,
            alpn: None,
            fingerprint: None,
            client_fingerprint: None,
            skip_cert_verify: None,
            reality_opts: None,
            network: None,
            smux: None,
            ws_opts: None,
            http_opts: None,
            h2_opts: None,
            grpc_opts: None,
        }
    }
}

impl From<Proxy> for VmessProxy {
    fn from(proxy: Proxy) -> Self {
        // 不在 CommonProxyOptions 中重复设置这些字段
        let common = CommonProxyOptions::builder(proxy.remark.clone(), proxy.hostname.clone(), proxy.port)
            .udp(proxy.udp)
            .tfo(proxy.tcp_fast_open)
            .build();

        let mut vmess = VmessProxy::new(common);

        // 基本配置
        vmess.uuid = proxy.user_id.clone();
        vmess.alter_id = proxy.alter_id as u32;
        vmess.cipher = proxy.encrypt_method.clone();
        vmess.network = proxy.transfer_protocol.clone();

        // TLS 配置 - 直接在 VmessProxy 中设置
        vmess.tls = Some(proxy.tls_secure);
        vmess.servername = proxy.sni.clone();
        vmess.skip_cert_verify = proxy.allow_insecure;

        // 如果有指纹信息，设置指纹
        if let Some(fingerprint) = proxy.fingerprint {
            vmess.fingerprint = Some(fingerprint);
        }

        // 网络特定配置
        if let Some(network) = &proxy.transfer_protocol {
            match network.as_str() {
                "ws" => {
                    let mut ws_opts = VmessWsOptions {
                        path: None,
                        headers: None,
                    };

                    if let Some(path) = &proxy.path {
                        ws_opts.path = Some(path.clone());
                    }

                    if let Some(host) = &proxy.host {
                        let mut headers = HashMap::new();
                        headers.insert("Host".to_string(), host.clone());
                        ws_opts.headers = Some(headers);
                    }

                    vmess.ws_opts = Some(ws_opts);
                }
                "http" => {
                    let mut http_opts = VmessHttpOptions {
                        path: None,
                        headers: None,
                    };

                    if let Some(path) = &proxy.path {
                        http_opts.path = Some(path.clone());
                    }

                    if let Some(host) = &proxy.host {
                        let mut headers = HashMap::new();
                        headers.insert("Host".to_string(), host.clone());
                        http_opts.headers = Some(headers);
                    }

                    vmess.http_opts = Some(http_opts);
                }
                "h2" => {
                    let mut h2_opts = VmessH2Options {
                        path: None,
                        host: None,
                    };

                    if let Some(path) = &proxy.path {
                        h2_opts.path = Some(path.clone());
                    }

                    if let Some(host) = &proxy.host {
                        h2_opts.host = Some(vec![host.clone()]);
                    }

                    vmess.h2_opts = Some(h2_opts);
                }
                "grpc" => {
                    let mut grpc_opts = VmessGrpcOptions { service_name: None };

                    if let Some(path) = &proxy.path {
                        grpc_opts.service_name = Some(path.clone());
                    }

                    vmess.grpc_opts = Some(grpc_opts);
                }
                _ => {}
            }
        }

        vmess
    }
}
