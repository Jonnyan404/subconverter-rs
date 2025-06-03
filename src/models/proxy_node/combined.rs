use serde::{Deserialize, Serialize};

use super::anytls::AnyTlsProxy;
use super::shadowsocks::ShadowsocksProxy;
use super::vless::VlessProxy;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "combined_type")]
pub enum CombinedProxy {
    Vless(VlessProxy),
    Shadowsocks(ShadowsocksProxy),
    AnyTls(AnyTlsProxy),
}
