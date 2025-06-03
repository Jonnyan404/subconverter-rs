use crate::models::Proxy;
use crate::parser::yaml::clash::clash_proxy_types::ClashProxyYamlInput;

use super::ClashYamlInput;

/// Parse Clash configuration from YAML string
///
/// This function is the Rust equivalent of the C++ `explodeClash` function.
/// The key improvements in this Rust implementation are:
/// 1. Type safety through enum variants in ClashProxyYamlInput
/// 2. Proper error handling with Result type
/// 3. Automatic deserialization using serde
/// 4. Cleaner pattern matching compared to C++ if/else chains
pub fn parse_clash_yaml(content: &str) -> Result<Vec<Proxy>, String> {
    let clash_input: ClashYamlInput = match serde_yaml::from_str(content) {
        Ok(input) => input,
        Err(e) => return Err(format!("Failed to parse Clash YAML: {}", e)),
    };

    let mut proxies = Vec::new();

    for proxy in clash_input.extract_proxies() {
        match proxy {
            ClashProxyYamlInput::Shadowsocks(ss) => {
                proxies.push(ss.into());
            }
            ClashProxyYamlInput::ShadowsocksR(ssr) => {
                proxies.push(ssr.into());
            }
            ClashProxyYamlInput::VMess(vmess) => {
                proxies.push(vmess.into());
            }
            ClashProxyYamlInput::Trojan(trojan) => {
                proxies.push(trojan.into());
            }
            ClashProxyYamlInput::Http(http) => {
                proxies.push(http.into());
            }
            ClashProxyYamlInput::Socks5(socks5) => {
                proxies.push(socks5.into());
            }
            ClashProxyYamlInput::Snell(snell) => {
                proxies.push(snell.into());
            }
            ClashProxyYamlInput::WireGuard(wg) => {
                proxies.push(wg.into());
            }
            ClashProxyYamlInput::Hysteria(hysteria) => {
                proxies.push(hysteria.into());
            }
            ClashProxyYamlInput::Hysteria2(hysteria2) => {
                proxies.push(hysteria2.into());
            }
            ClashProxyYamlInput::VLess(vless) => {
                proxies.push(vless.into());
            }
            ClashProxyYamlInput::AnyTls(anytls) => {
                proxies.push(anytls.into());
            }
            ClashProxyYamlInput::Unknown => {
                // Skip unknown proxy types
            }
        }
    }

    Ok(proxies)
}
