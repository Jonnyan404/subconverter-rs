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
        ss.plugin = proxy.plugin.clone();

        if let Some(plugin_opts) = proxy.plugin_option {
            // 添加调试输出
            println!("Debug - Original plugin_opts: '{}'", plugin_opts);
            
            let mut opts = HashMap::new();
            let plugin_name = proxy.plugin.as_deref().unwrap_or("");

            // 改进插件选项解析逻辑
            for opt in plugin_opts.split(';') {
                let opt = opt.trim();
                if opt.is_empty() {
                    continue;
                }
                
                println!("Debug - Processing opt: '{}'", opt);
                
                if let Some(eq_pos) = opt.find('=') {
                    let key = opt[..eq_pos].trim();
                    let value = opt[eq_pos + 1..].trim();
                    
                    println!("Debug - Key: '{}', Value: '{}'", key, value);
                    
                    // 根据插件类型进行特殊处理
                    match plugin_name {
                        "v2ray-plugin" | "gost-plugin" => {
                            match key {
                                "mode" => {
                                    // v2ray-plugin 和 gost-plugin 的 mode 参数保持原样
                                    opts.insert("mode".to_string(), value.to_string());
                                },
                                "host" => {
                                    opts.insert("host".to_string(), value.to_string());
                                },
                                "path" => {
                                    opts.insert("path".to_string(), value.to_string());
                                },
                                "tls" => {
                                    // 处理布尔值
                                    match value.to_lowercase().as_str() {
                                        "true" | "1" => opts.insert("tls".to_string(), "true".to_string()),
                                        "false" | "0" => opts.insert("tls".to_string(), "false".to_string()),
                                        _ => opts.insert("tls".to_string(), value.to_string()),
                                    };
                                },
                                "mux" => {
                                    // 处理布尔值
                                    match value.to_lowercase().as_str() {
                                        "true" | "1" => opts.insert("mux".to_string(), "true".to_string()),
                                        "false" | "0" => opts.insert("mux".to_string(), "false".to_string()),
                                        _ => opts.insert("mux".to_string(), value.to_string()),
                                    };
                                },
                                "skip-cert-verify" => {
                                    match value.to_lowercase().as_str() {
                                        "true" | "1" => opts.insert("skip-cert-verify".to_string(), "true".to_string()),
                                        "false" | "0" => opts.insert("skip-cert-verify".to_string(), "false".to_string()),
                                        _ => opts.insert("skip-cert-verify".to_string(), value.to_string()),
                                    };
                                },
                                "fingerprint" => {
                                    opts.insert("fingerprint".to_string(), value.to_string());
                                },
                                "v2ray-http-upgrade" => {
                                    match value.to_lowercase().as_str() {
                                        "true" | "1" => opts.insert("v2ray-http-upgrade".to_string(), "true".to_string()),
                                        "false" | "0" => opts.insert("v2ray-http-upgrade".to_string(), "false".to_string()),
                                        _ => opts.insert("v2ray-http-upgrade".to_string(), value.to_string()),
                                    };
                                },
                                _ => {
                                    // 其他参数直接添加
                                    opts.insert(key.to_string(), value.to_string());
                                }
                            }
                        },
                        "obfs" => {
                            match key {
                                "mode" => {
                                    opts.insert("mode".to_string(), value.to_string());
                                },
                                "host" => {
                                    opts.insert("host".to_string(), value.to_string());
                                },
                                _ => {
                                    opts.insert(key.to_string(), value.to_string());
                                }
                            }
                        },
                        "shadow-tls" => {
                            match key {
                                "host" => {
                                    opts.insert("host".to_string(), value.to_string());
                                },
                                "password" => {
                                    opts.insert("password".to_string(), value.to_string());
                                },
                                "version" => {
                                    // 尝试解析为数字
                                    if let Ok(ver) = value.parse::<u8>() {
                                        opts.insert("version".to_string(), ver.to_string());
                                    } else {
                                        opts.insert("version".to_string(), value.to_string());
                                    }
                                },
                                _ => {
                                    opts.insert(key.to_string(), value.to_string());
                                }
                            }
                        },
                        "restls" => {
                            match key {
                                "host" => {
                                    opts.insert("host".to_string(), value.to_string());
                                },
                                "password" => {
                                    opts.insert("password".to_string(), value.to_string());
                                },
                                "version-hint" => {
                                    opts.insert("version-hint".to_string(), value.to_string());
                                },
                                "restls-script" => {
                                    opts.insert("restls-script".to_string(), value.to_string());
                                },
                                _ => {
                                    opts.insert(key.to_string(), value.to_string());
                                }
                            }
                        },
                        _ => {
                            // 未知插件，按原样处理
                            opts.insert(key.to_string(), value.to_string());
                        }
                    }
                } else {
                    // 处理没有值的布尔选项
                    if !opt.is_empty() {
                        opts.insert(opt.to_string(), "true".to_string());
                    }
                }
            }

            println!("Debug - Final opts: {:?}", opts);

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
