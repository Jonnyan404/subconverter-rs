use serde::Deserialize;

use crate::models::proxy::Proxy;
use crate::models::proxy::ProxyType;
use crate::models::proxy_node::anytls::AnyTlsProxy;
use crate::models::proxy_node::combined::CombinedProxy;

/// Represents an AnyTLS proxy in Clash configuration (mihomo extension)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClashInputAnyTLS {
    name: String,
    server: String,
    port: u16,
    password: String,
    #[serde(default)]
    alpn: Option<Vec<String>>,
    #[serde(default)]
    sni: Option<String>,
    #[serde(default)]
    tfo: Option<bool>,
    #[serde(alias = "skip-cert-verify", default)]
    skip_cert_verify: Option<bool>,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(alias = "client-fingerprint", default)]
    client_fingerprint: Option<String>,
    #[serde(default)]
    udp: Option<bool>,
    #[serde(alias = "idle-session-check-interval", default)]
    idle_session_check_interval: Option<i32>,
    #[serde(alias = "idle-session-timeout", default)]
    idle_session_timeout: Option<i32>,
    #[serde(alias = "min-idle-session", default)]
    min_idle_session: Option<i32>,
}

impl Into<Proxy> for ClashInputAnyTLS {
    fn into(self) -> Proxy {
        let mut proxy = Proxy::default();
        proxy.proxy_type = ProxyType::AnyTls;

        let mut anytls_proxy = AnyTlsProxy::default();
        anytls_proxy.password = self.password;
        anytls_proxy.alpn = self.alpn.map(|v| v.into_iter().collect()); // Convert Vec to HashSet if needed later, or keep as Vec
        anytls_proxy.sni = self.sni;
        anytls_proxy.skip_cert_verify = self.skip_cert_verify;
        anytls_proxy.fingerprint = self.fingerprint;
        anytls_proxy.client_fingerprint = self.client_fingerprint;
        anytls_proxy.udp = self.udp; // Default to false? Check mihomo default
        anytls_proxy.idle_session_check_interval = self.idle_session_check_interval;
        anytls_proxy.idle_session_timeout = self.idle_session_timeout;
        anytls_proxy.min_idle_session = self.min_idle_session;
        anytls_proxy.tfo = self.tfo;

        proxy.combined_proxy = Some(CombinedProxy::AnyTls(anytls_proxy));
        proxy.remark = self.name;
        proxy.hostname = self.server;
        proxy.port = self.port;
        proxy.tcp_fast_open = self.tfo;
        proxy.udp = self.udp;

        proxy
    }
}
