use super::CommonProxyOptions;
use crate::models::Proxy;
use crate::utils::is_empty_option_string;
use crate::utils::url::url_decode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Shadowsocks proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShadowsocksProxy {
    #[serde(flatten)]
    pub common: CommonProxyOptions,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub cipher: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "is_empty_option_string")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_opts: Option<HashMap<String, String>>,
    // Additional fields from the C++ implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp_version: Option<u8>,
    // Fields from the SingBox implementation
    // pub network: Option<String>, // Similar to NetworkList in SingBox
    // pub multiplex: Option<HashMap<String, bool>>, // OutboundMultiplexOptions

    // Fields from the ClashMeta implementation
    // pub client_fingerprint: Option<String>,

    // These fields would be in common options:
    // - udp (already implemented)
    // - tfo (already implemented as tcp_fast_open)
    // - skip_cert_verify (already implemented)
    // - mptcp (not implemented yet)
    // - interface (not implemented yet)
    // - routing_mark (not implemented yet)
    // - ip_version (not implemented yet)
    // - dialer_proxy (not implemented yet)
}

impl ShadowsocksProxy {
    /// Create a new Shadowsocks proxy
    pub fn new(common: CommonProxyOptions) -> Self {
        Self {
            common,
            cipher: None,
            password: None,
            plugin: None,
            plugin_opts: None,
            udp_over_tcp: None,
            udp_over_tcp_version: None,
        }
    }
}

impl From<Proxy> for ShadowsocksProxy {
    fn from(proxy: Proxy) -> Self {
        let common =
            CommonProxyOptions::builder(proxy.remark.clone(), proxy.hostname.clone(), proxy.port)
                .udp(proxy.udp)
                .tfo(proxy.tcp_fast_open)
                .skip_cert_verify(proxy.allow_insecure)
                .sni(proxy.sni.clone())
                .build();

        let mut ss = ShadowsocksProxy::new(common);

        ss.cipher = proxy.encrypt_method;
        ss.password = proxy.password.map(|pwd| url_decode(&pwd));
        ss.plugin = proxy.plugin;

        if let Some(plugin_opts) = proxy.plugin_option {
            let mut opts = HashMap::new();

            // 改进插件选项解析逻辑
            for opt in plugin_opts.split(';') {
                if opt.trim().is_empty() {
                    continue;
                }
                
                let parts: Vec<&str> = opt.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim();
                    
                    // 处理特殊的映射关系
                    match key {
                        "mode" => {
                            // v2ray-plugin 的 mode 参数映射到 obfs
                            if value == "websocket" {
                                opts.insert("obfs".to_string(), "websocket".to_string());
                            } else {
                                opts.insert("obfs".to_string(), value.to_string());
                            }
                        },
                        "host" => {
                            opts.insert("host".to_string(), value.to_string());
                        },
                        "path" => {
                            opts.insert("path".to_string(), value.to_string());
                        },
                        "tls" => {
                            // 处理布尔值
                            if value == "true" || value == "1" {
                                opts.insert("tls".to_string(), "true".to_string());
                            } else if value == "false" || value == "0" {
                                opts.insert("tls".to_string(), "false".to_string());
                            } else {
                                opts.insert("tls".to_string(), value.to_string());
                            }
                        },
                        "mux" => {
                            // 处理布尔值
                            if value == "true" || value == "1" {
                                opts.insert("mux".to_string(), "true".to_string());
                            } else if value == "false" || value == "0" {
                                opts.insert("mux".to_string(), "false".to_string());
                            } else {
                                opts.insert("mux".to_string(), value.to_string());
                            }
                        },
                        _ => {
                            // 其他参数直接添加
                            opts.insert(key.to_string(), value.to_string());
                        }
                    }
                } else if parts.len() == 1 && !parts[0].trim().is_empty() {
                    // 处理没有值的布尔选项
                    let key = parts[0].trim();
                    opts.insert(key.to_string(), "true".to_string());
                }
            }

            if !opts.is_empty() {
                ss.plugin_opts = Some(opts);
            }
        }

        // Map combined_proxy fields if available
        if let Some(ref combined) = proxy.combined_proxy {
            if let crate::models::proxy_node::combined::CombinedProxy::Shadowsocks(ref ss_proxy) =
                combined
            {
                ss.udp_over_tcp = ss_proxy.udp_over_tcp;
                ss.udp_over_tcp_version = ss_proxy.udp_over_tcp_version;
            }
        }

        ss
    }
}
