use super::CommonProxyOptions;
use crate::models::Proxy;
use crate::utils::is_empty_option_string;
use crate::utils::url::url_decode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub plugin_opts: Option<HashMap<String, Value>>, // 改为支持不同类型的值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp_version: Option<u8>,
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
            println!("Debug - Original plugin_opts: '{}'", plugin_opts);
            
            let mut opts = HashMap::new();
            let plugin_name = proxy.plugin.as_deref().unwrap_or("");

            // 改进插件选项解析逻辑，添加格式适配
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
                    
                    // 根据插件类型进行格式适配和处理
                    match plugin_name {
                        "v2ray-plugin" | "gost-plugin" => {
                            match key {
                                // 适配 obfs=websocket -> mode=websocket
                                "obfs" if value == "websocket" => {
                                    opts.insert("mode".to_string(), Value::String("websocket".to_string()));
                                },
                                "mode" => {
                                    opts.insert("mode".to_string(), Value::String(value.to_string()));
                                },
                                "host" => {
                                    opts.insert("host".to_string(), Value::String(value.to_string()));
                                },
                                "path" => {
                                    opts.insert("path".to_string(), Value::String(value.to_string()));
                                },
                                "tls" => {
                                    let bool_value = match value.to_lowercase().as_str() {
                                        "true" | "1" => true,
                                        "false" | "0" => false,
                                        _ => value.parse().unwrap_or(false),
                                    };
                                    opts.insert("tls".to_string(), Value::Bool(bool_value));
                                },
                                "mux" => {
                                    let bool_value = match value.to_lowercase().as_str() {
                                        "true" | "1" => true,
                                        "false" | "0" => false,
                                        _ => value.parse().unwrap_or(false),
                                    };
                                    opts.insert("mux".to_string(), Value::Bool(bool_value));
                                },
                                "skip-cert-verify" => {
                                    let bool_value = match value.to_lowercase().as_str() {
                                        "true" | "1" => true,
                                        "false" | "0" => false,
                                        _ => value.parse().unwrap_or(false),
                                    };
                                    opts.insert("skip-cert-verify".to_string(), Value::Bool(bool_value));
                                },
                                "fingerprint" => {
                                    opts.insert("fingerprint".to_string(), Value::String(value.to_string()));
                                },
                                "v2ray-http-upgrade" => {
                                    let bool_value = match value.to_lowercase().as_str() {
                                        "true" | "1" => true,
                                        "false" | "0" => false,
                                        _ => value.parse().unwrap_or(false),
                                    };
                                    opts.insert("v2ray-http-upgrade".to_string(), Value::Bool(bool_value));
                                },
                                _ => {
                                    // 其他参数直接添加为字符串
                                    opts.insert(key.to_string(), Value::String(value.to_string()));
                                }
                            }
                        },
                        "obfs" => {
                            match key {
                                "mode" => {
                                    opts.insert("mode".to_string(), Value::String(value.to_string()));
                                },
                                "host" => {
                                    opts.insert("host".to_string(), Value::String(value.to_string()));
                                },
                                _ => {
                                    opts.insert(key.to_string(), Value::String(value.to_string()));
                                }
                            }
                        },
                        _ => {
                            // 对于未知插件，也进行基本的格式适配
                            match key {
                                "obfs" if value == "websocket" => {
                                    opts.insert("mode".to_string(), Value::String("websocket".to_string()));
                                },
                                "tls" | "mux" | "skip-cert-verify" => {
                                    let bool_value = match value.to_lowercase().as_str() {
                                        "true" | "1" => true,
                                        "false" | "0" => false,
                                        _ => value.parse().unwrap_or(false),
                                    };
                                    opts.insert(key.to_string(), Value::Bool(bool_value));
                                },
                                _ => {
                                    opts.insert(key.to_string(), Value::String(value.to_string()));
                                }
                            }
                        }
                    }
                } else {
                    // 处理没有值的布尔选项和特殊格式
                    if opt.starts_with("obfs-host") && opt.len() > 9 {
                        // 处理 obfs-hostwt8v1.kvote.cn -> host=wt8v1.kvote.cn
                        let host_value = &opt[9..]; // 移除 "obfs-host" 前缀
                        println!("Debug - Fixed obfs-host: '{}'", host_value);
                        opts.insert("host".to_string(), Value::String(host_value.to_string()));
                    } else if opt == "tls" || opt == "mux" {
                        // 单独的 tls/mux 表示启用
                        opts.insert(opt.to_string(), Value::Bool(true));
                    } else if !opt.is_empty() {
                        // 其他没有值的选项当作布尔值
                        opts.insert(opt.to_string(), Value::Bool(true));
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
