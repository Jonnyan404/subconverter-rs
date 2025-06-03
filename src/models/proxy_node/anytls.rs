use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents the AnyTLS proxy details
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AnyTlsProxy {
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<HashSet<String>>, // Using HashSet for uniqueness, similar to vless alpn
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_session_check_interval: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_session_timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_idle_session: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfo: Option<bool>,
}
