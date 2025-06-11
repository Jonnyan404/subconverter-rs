use crate::{Proxy, ProxyType};

impl Proxy {
    pub fn common_construct(
        proxy_type: ProxyType,
        group: &str,
        remark: &str,
        server: &str,
        port: u16,
        udp: Option<bool>,
        tfo: Option<bool>,
        scv: Option<bool>,
        tls13: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        Proxy {
            proxy_type,
            group: group.to_owned(),
            remark: remark.to_owned(),
            hostname: server.to_owned(),
            port,
            udp,
            tcp_fast_open: tfo,
            allow_insecure: scv,
            tls13,
            underlying_proxy: Some(underlying_proxy.to_owned()),
            ..Default::default()
        }
    }

    pub fn vmess_construct(
        group: &str,
        remark: &str,
        add: &str,
        port: u16,
        typ: &str,
        id: &str,
        aid: u16,
        net: &str,
        cipher: &str,
        path: &str,
        host: &str,
        edge: &str,
        tls: &str,
        sni: &str,
        udp: Option<bool>,
        tfo: Option<bool>,
        scv: Option<bool>,
        tls13: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Proxy::common_construct(
            ProxyType::VMess,
            group,
            remark,
            add,
            port,
            udp,
            tfo,
            scv,
            tls13,
            underlying_proxy,
        );
        proxy.user_id = if id.is_empty() {
            Some("00000000-0000-0000-0000-000000000000".to_owned())
        } else {
            Some(id.to_owned())
        };
        proxy.alter_id = aid;
        proxy.encrypt_method = if cipher.is_empty() {
            None
        } else {
            Some(cipher.to_owned())
        };
        proxy.transfer_protocol = Some(if net.is_empty() { "tcp" } else { net }.to_owned());
        proxy.edge = if edge.is_empty() {
            None
        } else {
            Some(edge.to_owned())
        };
        proxy.server_name = if sni.is_empty() {
            None
        } else {
            Some(sni.to_owned())
        };
        proxy.tls_secure = tls == "tls";

        if net == "quic" {
            proxy.quic_secure = Some(host.to_owned());
            proxy.quic_secret = Some(path.to_owned());
        } else {
            proxy.host = Some(
                if host.is_empty() && !add.parse::<std::net::IpAddr>().is_ok() {
                    add.to_owned()
                } else {
                    host.trim().to_owned()
                },
            );
            proxy.path = Some(if path.is_empty() { "/" } else { path.trim() }.to_owned());
        }
        proxy.fake_type = Some(typ.to_owned());

        proxy
    }

    pub fn ssr_construct(
        group: &str,
        remark: &str,
        server: &str,
        port: u16,
        protocol: &str,
        method: &str,
        obfs: &str,
        password: &str,
        obfs_param: &str,
        proto_param: &str,
        udp: Option<bool>,
        tfo: Option<bool>,
        scv: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Proxy::common_construct(
            ProxyType::ShadowsocksR,
            group,
            remark,
            server,
            port,
            udp,
            tfo,
            scv,
            None,
            underlying_proxy,
        );
        proxy.password = Some(password.to_owned());
        proxy.encrypt_method = Some(method.to_owned());
        proxy.protocol = Some(protocol.to_owned());
        proxy.protocol_param = Some(proto_param.to_owned());
        proxy.obfs = Some(obfs.to_owned());
        proxy.obfs_param = Some(obfs_param.to_owned());

        proxy
    }

    pub fn ss_construct(
        group: &str,
        remark: &str,
        server: &str,
        port: u16,
        password: &str,
        method: &str,
        plugin: &str,
        plugin_opts: &str,
        udp: Option<bool>,
        tfo: Option<bool>,
        scv: Option<bool>,
        tls13: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Proxy::common_construct(
            ProxyType::Shadowsocks,
            group,
            remark,
            server,
            port,
            udp,
            tfo,
            scv,
            tls13,
            underlying_proxy,
        );

        // Set up the combined proxy with ShadowsocksProxy
        let ss_proxy = crate::models::proxy_node::shadowsocks::ShadowsocksProxy {
            server: server.to_string(),
            port,
            password: password.to_string(),
            cipher: method.to_string(),
            udp,
            tfo,
            skip_cert_verify: scv,
            plugin: if plugin.is_empty() {
                None
            } else {
                Some(plugin.to_string())
            },
            plugin_opts: if plugin_opts.is_empty() {
                None
            } else {
                Some(plugin_opts.to_string())
            },
            udp_over_tcp: None,
            udp_over_tcp_version: None,
            client_fingerprint: None,
        };

        proxy.combined_proxy =
            Some(crate::models::proxy_node::combined::CombinedProxy::Shadowsocks(ss_proxy));

        // Keep the old fields for backward compatibility
        proxy.password = Some(password.to_owned());
        proxy.encrypt_method = Some(method.to_owned());
        proxy.plugin = Some(plugin.to_owned());
        proxy.plugin_option = Some(plugin_opts.to_owned());

        proxy
    }

    pub fn socks_construct(
        group: &str,
        remark: &str,
        server: &str,
        port: u16,
        username: &str,
        password: &str,
        udp: Option<bool>,
        tfo: Option<bool>,
        scv: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Proxy::common_construct(
            ProxyType::Socks5,
            group,
            remark,
            server,
            port,
            udp,
            tfo,
            scv,
            None,
            underlying_proxy,
        );
        proxy.username = Some(username.to_owned());
        proxy.password = Some(password.to_owned());

        proxy
    }

    pub fn http_construct(
        group: &str,
        remark: &str,
        server: &str,
        port: u16,
        username: &str,
        password: &str,
        tls: bool,
        tfo: Option<bool>,
        scv: Option<bool>,
        tls13: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Proxy::common_construct(
            if tls {
                ProxyType::HTTPS
            } else {
                ProxyType::HTTP
            },
            group,
            remark,
            server,
            port,
            None,
            tfo,
            scv,
            tls13,
            underlying_proxy,
        );
        proxy.username = Some(username.to_owned());
        proxy.password = Some(password.to_owned());
        proxy.tls_secure = tls;

        proxy
    }

    pub fn trojan_construct(
        group: String,
        remark: String,
        hostname: String,
        port: u16,
        password: String,
        network: Option<String>,
        host: Option<String>,
        path: Option<String>,
        sni: Option<String>,
        tls_secure: bool,
        udp: Option<bool>,
        tfo: Option<bool>,
        allow_insecure: Option<bool>,
        tls13: Option<bool>,
        underlying_proxy: Option<String>,
    ) -> Self {
        Proxy {
            proxy_type: ProxyType::Trojan,
            group,
            remark,
            hostname,
            port,
            password: Some(password),
            transfer_protocol: network,
            host,
            path,
            sni,
            tls_secure,
            udp,
            tcp_fast_open: tfo,
            allow_insecure,
            tls13,
            underlying_proxy,
            ..Default::default()
        }
    }

    pub fn snell_construct(
        group: String,
        remark: String,
        hostname: String,
        port: u16,
        password: String,
        obfs: String,
        host: String,
        version: u16,
        udp: Option<bool>,
        tfo: Option<bool>,
        allow_insecure: Option<bool>,
        underlying_proxy: Option<String>,
    ) -> Self {
        Proxy {
            proxy_type: ProxyType::Snell,
            group,
            remark,
            hostname,
            port,
            password: Some(password),
            obfs: Some(obfs),
            host: Some(host),
            snell_version: version,
            udp,
            tcp_fast_open: tfo,
            allow_insecure,
            underlying_proxy,
            ..Default::default()
        }
    }

    pub fn wireguard_construct(
        group: String,
        remark: String,
        hostname: String,
        port: u16,
        self_ip: String,
        self_ipv6: String,
        private_key: String,
        public_key: String,
        preshared_key: String,
        dns_servers: Vec<String>,
        mtu: Option<u16>,
        keep_alive: Option<u16>,
        test_url: String,
        client_id: String,
        udp: Option<bool>,
        underlying_proxy: Option<String>,
    ) -> Self {
        let mut dns_set = std::collections::HashSet::new();
        for dns in dns_servers {
            dns_set.insert(dns);
        }

        Proxy {
            proxy_type: ProxyType::WireGuard,
            group,
            remark,
            hostname,
            port,
            self_ip: Some(self_ip),
            self_ipv6: Some(self_ipv6),
            private_key: Some(private_key),
            public_key: Some(public_key),
            pre_shared_key: Some(preshared_key),
            dns_servers: dns_set,
            mtu: mtu.unwrap_or(0),
            keep_alive: keep_alive.unwrap_or(0),
            test_url: Some(test_url),
            client_id: Some(client_id),
            udp,
            underlying_proxy,
            ..Default::default()
        }
    }

    pub fn hysteria2_construct(
        group: String,
        remark: String,
        hostname: String,
        port: u16,
        ports: Option<String>,
        up_speed: Option<u32>,
        down_speed: Option<u32>,
        password: String,
        obfs: Option<String>,
        obfs_param: Option<String>,
        sni: Option<String>,
        fingerprint: Option<String>,
        alpn: Vec<String>,
        ca: Option<String>,
        ca_str: Option<String>,
        cwnd: Option<u32>,
        tcp_fast_open: Option<bool>,
        allow_insecure: Option<bool>,
        underlying_proxy: Option<String>,
    ) -> Self {
        let mut alpn_set = std::collections::HashSet::new();
        for proto in alpn {
            alpn_set.insert(proto);
        }

        Proxy {
            proxy_type: ProxyType::Hysteria2,
            group,
            remark,
            hostname,
            port,
            ports,
            up_speed: up_speed.unwrap_or(0),
            down_speed: down_speed.unwrap_or(0),
            password: Some(password),
            obfs: obfs,
            obfs_param: obfs_param,
            sni: sni,
            fingerprint: fingerprint,
            alpn: alpn_set,
            ca: ca,
            ca_str: ca_str,
            cwnd: cwnd.unwrap_or(0),
            tcp_fast_open,
            allow_insecure,
            underlying_proxy,
            ..Default::default()
        }
    }

    pub fn hysteria_construct(
        group: String,
        remark: String,
        hostname: String,
        port: u16,
        ports: String,
        protocol: String,
        obfs_param: String,
        up_speed: Option<u32>,
        down_speed: Option<u32>,
        auth_str: String,
        obfs: String,
        sni: String,
        fingerprint: String,
        ca: String,
        ca_str: String,
        recv_window_conn: Option<u32>,
        recv_window: Option<u32>,
        disable_mtu_discovery: Option<bool>,
        hop_interval: Option<u32>,
        alpn: Vec<String>,
        tcp_fast_open: Option<bool>,
        allow_insecure: Option<bool>,
        underlying_proxy: Option<String>,
    ) -> Self {
        let mut alpn_set = std::collections::HashSet::new();
        for proto in alpn {
            alpn_set.insert(proto);
        }

        Proxy {
            proxy_type: ProxyType::Hysteria,
            group,
            remark,
            hostname,
            port,
            ports: Some(ports),
            protocol: Some(protocol),
            obfs_param: Some(obfs_param),
            up_speed: up_speed.unwrap_or(0),
            down_speed: down_speed.unwrap_or(0),
            auth_str: Some(auth_str),
            obfs: Some(obfs),
            sni: Some(sni),
            fingerprint: Some(fingerprint),
            ca: Some(ca),
            ca_str: Some(ca_str),
            recv_window_conn: recv_window_conn.unwrap_or(0),
            recv_window: recv_window.unwrap_or(0),
            disable_mtu_discovery,
            hop_interval: hop_interval.unwrap_or(0),
            alpn: alpn_set,
            tcp_fast_open,
            allow_insecure,
            underlying_proxy,
            ..Default::default()
        }
    }

    pub fn vless_construct(
        group: &str,
        name: &str,
        server: &str,
        port: u16,
        uuid: &str,
        flow: Option<String>,
        network: &str,
        tls: &str,
        servername: &str,
        reality_public_key: Option<String>,
        reality_short_id: Option<String>,
        packet_encoding: Option<String>,
        client_fingerprint: Option<String>,
        fingerprint: Option<String>,
        alpn: Option<Vec<String>>,
        grpc_service_name: Option<String>,
        ws_path: Option<String>,
        ws_headers: Option<std::collections::HashMap<String, String>>,
        h2_host: Option<Vec<String>>,
        h2_path: Option<String>,
        smux_enabled: Option<bool>,
        smux_protocol: Option<String>,
        smux_padding: Option<bool>,
        smux_max_connections: Option<String>,
        smux_min_streams: Option<String>,
        smux_statistic: Option<bool>,
        smux_only_tcp: Option<bool>,
        brutal_enabled: Option<bool>,
        brutal_up: Option<String>,
        brutal_down: Option<String>,
        udp: Option<bool>,
        tfo: Option<bool>,
        skip_cert_verify: Option<bool>,
        underlying_proxy: &str,
    ) -> Self {
        let mut proxy = Self::common_construct(
            crate::ProxyType::Vless,
            group,
            name,
            server,
            port,
            udp,
            tfo,
            skip_cert_verify,
            None,
            underlying_proxy,
        );

        // ✅ 只设置直接字段，不设置 combined_proxy
        proxy.user_id = Some(uuid.to_string());
        proxy.transfer_protocol = Some(network.to_string());
        proxy.tls_secure = tls == "tls";
        proxy.server_name = if !servername.is_empty() { 
            Some(servername.to_string()) 
        } else { 
            None 
        };
        proxy.public_key = reality_public_key;
        proxy.reality_short_id = reality_short_id;
        proxy.flow = flow;
        proxy.packet_encoding = packet_encoding;
        proxy.client_fingerprint = client_fingerprint;
        proxy.fingerprint = fingerprint;
        proxy.grpc_service_name = grpc_service_name;
        proxy.ws_headers = ws_headers;
        proxy.path = ws_path.or(h2_path);
        proxy.host = h2_host.and_then(|hosts| hosts.into_iter().next());
        proxy.smux_enabled = smux_enabled;
        proxy.smux_protocol = smux_protocol;
        proxy.smux_padding = smux_padding;
        proxy.smux_max_connections = smux_max_connections;
        proxy.smux_min_streams = smux_min_streams;
        proxy.smux_statistic = smux_statistic;
        proxy.smux_only_tcp = smux_only_tcp;
        proxy.brutal_enabled = brutal_enabled;
        proxy.brutal_up = brutal_up;
        proxy.brutal_down = brutal_down;

        if let Some(alpn_vec) = alpn {
            proxy.alpn = alpn_vec.into_iter().collect();
        }

        proxy
    }
}
