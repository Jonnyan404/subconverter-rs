// Conversion implementation for Settings struct

use std::collections::HashMap;

use super::ini_settings::IniSettings;
use super::settings_struct::{default_listen_address, Settings};
use super::toml_settings::TomlSettings;
use super::yaml_settings::YamlSettings;

use crate::constants::log_level::{
    LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_VERBOSE, LOG_LEVEL_WARNING,
};

// Conversion from YamlSettings to Settings
impl From<YamlSettings> for Settings {
    fn from(yaml_settings: YamlSettings) -> Self {
        let mut settings = Settings::default();

        // Common settings
        settings.default_ext_config = yaml_settings.common.default_external_config;
        settings.exclude_remarks = yaml_settings.common.exclude_remarks;
        settings.include_remarks = yaml_settings.common.include_remarks;
        settings.enable_filter = yaml_settings.common.enable_filter;
        settings.filter_script = yaml_settings.common.filter_script;
        settings.api_mode = yaml_settings.common.api_mode;
        settings.api_access_token = yaml_settings.common.api_access_token;
        settings.base_path = yaml_settings.common.base_path;
        settings.clash_base = yaml_settings.common.clash_rule_base;
        settings.surge_base = yaml_settings.common.surge_rule_base;
        settings.surfboard_base = yaml_settings.common.surfboard_rule_base;
        settings.mellow_base = yaml_settings.common.mellow_rule_base;
        settings.quan_base = yaml_settings.common.quan_rule_base;
        settings.quanx_base = yaml_settings.common.quanx_rule_base;
        settings.loon_base = yaml_settings.common.loon_rule_base;
        settings.ssub_base = yaml_settings.common.sssub_rule_base;
        settings.singbox_base = yaml_settings.common.singbox_rule_base;
        settings.proxy_config = yaml_settings.common.proxy_config;
        settings.proxy_ruleset = yaml_settings.common.proxy_ruleset;
        settings.proxy_subscription = yaml_settings.common.proxy_subscription;
        settings.append_type = yaml_settings.common.append_proxy_type;
        settings.reload_conf_on_request = yaml_settings.common.reload_conf_on_request;

        // Process default_url and insert_url
        if !yaml_settings.common.default_url.is_empty() {
            settings.default_urls = yaml_settings.common.default_url;
        }

        if !yaml_settings.common.insert_url.is_empty() {
            settings.insert_urls = yaml_settings.common.insert_url;
            settings.enable_insert = yaml_settings.common.enable_insert;
            settings.prepend_insert = yaml_settings.common.prepend_insert_url;
        }

        // Node preferences
        settings.udp_flag = yaml_settings.node_pref.udp_flag;
        settings.tfo_flag = yaml_settings.node_pref.tcp_fast_open_flag;
        settings.skip_cert_verify = yaml_settings.node_pref.skip_cert_verify_flag;
        settings.tls13_flag = yaml_settings.node_pref.tls13_flag;
        settings.enable_sort = yaml_settings.node_pref.sort_flag;
        settings.sort_script = yaml_settings.node_pref.sort_script;
        settings.filter_deprecated = yaml_settings.node_pref.filter_deprecated_nodes;
        settings.append_userinfo = yaml_settings.node_pref.append_sub_userinfo;
        settings.clash_use_new_field = yaml_settings.node_pref.clash_use_new_field_name;
        settings.clash_proxies_style = yaml_settings.node_pref.clash_proxies_style;
        settings.clash_proxy_groups_style = yaml_settings.node_pref.clash_proxy_groups_style;
        settings.singbox_add_clash_modes = yaml_settings.node_pref.singbox_add_clash_modes;
        // Managed config
        settings.write_managed_config = yaml_settings.managed_config.write_managed_config;
        settings.managed_config_prefix = yaml_settings.managed_config.managed_config_prefix;
        settings.update_interval = yaml_settings.managed_config.config_update_interval;
        settings.update_strict = yaml_settings.managed_config.config_update_strict;
        settings.quanx_dev_id = yaml_settings.managed_config.quanx_device_id;

        // Surge external proxy
        settings.surge_ssr_path = yaml_settings.surge_external_proxy.surge_ssr_path;
        settings.surge_resolve_hostname = yaml_settings.surge_external_proxy.resolve_hostname;

        // Emoji
        settings.add_emoji = yaml_settings.emojis.add_emoji;
        settings.remove_emoji = yaml_settings.emojis.remove_old_emoji;

        // Server
        settings.listen_address = yaml_settings.server.listen;
        settings.listen_port = yaml_settings.server.port;

        // Advanced
        settings.log_level = match yaml_settings.advanced.log_level.as_str() {
            "debug" => LOG_LEVEL_DEBUG,
            "info" => LOG_LEVEL_INFO,
            "warning" => LOG_LEVEL_WARNING,
            "error" => LOG_LEVEL_ERROR,
            "verbose" => LOG_LEVEL_VERBOSE,
            _ => LOG_LEVEL_INFO,
        };
        if yaml_settings.advanced.print_debug_info {
            settings.log_level = LOG_LEVEL_VERBOSE;
        }
        settings.max_pending_conns = yaml_settings.advanced.max_pending_connections;
        settings.max_concur_threads = yaml_settings.advanced.max_concurrent_threads;
        settings.max_allowed_rulesets = yaml_settings.advanced.max_allowed_rulesets;
        settings.max_allowed_rules = yaml_settings.advanced.max_allowed_rules;
        settings.max_allowed_download_size = yaml_settings.advanced.max_allowed_download_size;
        settings.cache_subscription = yaml_settings.advanced.cache_subscription;
        settings.cache_config = yaml_settings.advanced.cache_config;
        settings.cache_ruleset = yaml_settings.advanced.cache_ruleset;
        settings.script_clean_context = yaml_settings.advanced.script_clean_context;
        settings.async_fetch_ruleset = yaml_settings.advanced.async_fetch_ruleset;
        settings.skip_failed_links = yaml_settings.advanced.skip_failed_links;

        // Template
        settings.template_path = yaml_settings.template.template_path;
        settings.template_vars = yaml_settings.template.globals;
        settings.template_vars = HashMap::new();

        // Ruleset settings
        settings.enable_rule_gen = yaml_settings.rulesets.enabled;
        settings.overwrite_original_rules = yaml_settings.rulesets.overwrite_original_rules;
        settings.update_ruleset_on_request = yaml_settings.rulesets.update_ruleset_on_request;

        // update fields processed
        settings.renames = yaml_settings.parsed_rename;
        settings.stream_rules = yaml_settings.parsed_stream_rule;
        settings.time_rules = yaml_settings.parsed_time_rule;
        settings.emojis = yaml_settings.parsed_emoji_rules;
        settings.custom_proxy_groups = yaml_settings.parsed_proxy_group;
        settings.custom_rulesets = yaml_settings.parsed_ruleset;
        settings.cron_tasks = yaml_settings.parsed_tasks;

        settings
    }
}

// Conversion from TomlSettings to Settings
impl From<TomlSettings> for Settings {
    fn from(toml_settings: TomlSettings) -> Self {
        let mut settings = Settings::default();

        // Common section
        let common = toml_settings.common;
        settings.default_ext_config = common.default_external_config;
        settings.exclude_remarks = common.exclude_remarks;
        settings.include_remarks = common.include_remarks;
        settings.api_mode = common.api_mode;
        settings.api_access_token = common.api_access_token;
        settings.base_path = common.base_path;
        settings.clash_base = common.clash_rule_base;
        settings.surge_base = common.surge_rule_base;
        settings.surfboard_base = common.surfboard_rule_base;
        settings.mellow_base = common.mellow_rule_base;
        settings.quan_base = common.quan_rule_base;
        settings.quanx_base = common.quanx_rule_base;
        settings.loon_base = common.loon_rule_base;
        settings.ssub_base = common.sssub_rule_base;
        settings.singbox_base = common.singbox_rule_base;
        settings.proxy_config = common.proxy_config;
        settings.proxy_ruleset = common.proxy_ruleset;
        settings.proxy_subscription = common.proxy_subscription;
        settings.append_type = common.append_proxy_type;
        settings.reload_conf_on_request = common.reload_conf_on_request;

        settings.enable_filter = common.enable_filter;
        settings.filter_script = common.filter_script;

        // Process default_url and insert_url
        if !common.default_urls.is_empty() {
            settings.default_urls = common.default_urls;
        }

        if !common.insert_urls.is_empty() {
            settings.insert_urls = common.insert_urls;
            settings.enable_insert = common.enable_insert;
            settings.prepend_insert = common.prepend_insert_url;
        }

        // Node preferences
        let node_pref = &toml_settings.node_pref;
        settings.udp_flag = node_pref.udp_flag;
        settings.tfo_flag = node_pref.tcp_fast_open_flag;
        settings.skip_cert_verify = node_pref.skip_cert_verify_flag;
        settings.tls13_flag = node_pref.tls13_flag;
        settings.enable_sort = node_pref.sort_flag;
        settings.sort_script = node_pref.sort_script.clone();
        settings.filter_deprecated = node_pref.filter_deprecated_nodes;
        settings.append_userinfo = node_pref.append_sub_userinfo;
        settings.clash_use_new_field = node_pref.clash_use_new_field_name;
        settings.clash_proxies_style = node_pref.clash_proxies_style.clone();
        settings.clash_proxy_groups_style = node_pref.clash_proxy_groups_style.clone();
        settings.singbox_add_clash_modes = node_pref.singbox_add_clash_modes;

        // Managed config
        settings.write_managed_config = toml_settings.managed_config.write_managed_config;
        settings.managed_config_prefix = toml_settings.managed_config.managed_config_prefix.clone();
        settings.update_interval = toml_settings.managed_config.config_update_interval;
        settings.update_strict = toml_settings.managed_config.config_update_strict;
        settings.quanx_dev_id = toml_settings.managed_config.quanx_device_id.clone();

        // Surge external proxy
        settings.surge_ssr_path = toml_settings.surge_external_proxy.surge_ssr_path.clone();
        settings.surge_resolve_hostname = toml_settings.surge_external_proxy.resolve_hostname;

        // Emoji
        settings.add_emoji = toml_settings.emojis.add_emoji;
        settings.remove_emoji = toml_settings.emojis.remove_old_emoji;

        // Server
        settings.listen_address = toml_settings.server.listen.clone();
        settings.listen_port = toml_settings.server.port;

        // Advanced
        let log_level = &toml_settings.advanced.log_level;
        settings.log_level = match log_level.as_str() {
            "debug" => LOG_LEVEL_DEBUG,
            "info" => LOG_LEVEL_INFO,
            "warning" => LOG_LEVEL_WARNING,
            "error" => LOG_LEVEL_ERROR,
            "verbose" => LOG_LEVEL_VERBOSE,
            _ => LOG_LEVEL_INFO,
        };
        if toml_settings.advanced.print_debug_info {
            settings.log_level = LOG_LEVEL_VERBOSE;
        }
        settings.max_pending_conns = toml_settings.advanced.max_pending_connections;
        settings.max_concur_threads = toml_settings.advanced.max_concurrent_threads;
        settings.max_allowed_rulesets = toml_settings.advanced.max_allowed_rulesets;
        settings.max_allowed_rules = toml_settings.advanced.max_allowed_rules;
        settings.max_allowed_download_size = toml_settings.advanced.max_allowed_download_size;
        settings.cache_subscription = toml_settings.advanced.cache_subscription;
        settings.cache_config = toml_settings.advanced.cache_config;
        settings.cache_ruleset = toml_settings.advanced.cache_ruleset;
        settings.script_clean_context = toml_settings.advanced.script_clean_context;
        settings.async_fetch_ruleset = toml_settings.advanced.async_fetch_ruleset;
        settings.skip_failed_links = toml_settings.advanced.skip_failed_links;

        // Template
        settings.template_path = toml_settings.template.template_path.clone();
        settings.template_vars = toml_settings.template.globals;

        // Ruleset settings
        if !toml_settings.rulesets.is_empty() && toml_settings.rulesets[0].ruleset.is_some() {
            settings.enable_rule_gen = toml_settings.ruleset.enabled;
            settings.overwrite_original_rules = toml_settings.ruleset.overwrite_original_rules;
            settings.update_ruleset_on_request = toml_settings.ruleset.update_ruleset_on_request;
        }

        // Ensure listen_address is not empty
        if settings.listen_address.trim().is_empty() {
            settings.listen_address = default_listen_address();
        }

        // update fields processed
        settings.renames = toml_settings.parsed_rename;
        settings.stream_rules = toml_settings.parsed_stream_rule;
        settings.time_rules = toml_settings.parsed_time_rule;
        settings.emojis = toml_settings.parsed_emoji_rules;
        settings.custom_proxy_groups = toml_settings.parsed_proxy_group;
        settings.custom_rulesets = toml_settings.parsed_ruleset;
        settings.cron_tasks = toml_settings.parsed_tasks;

        settings
    }
}

// Conversion from IniSettings to Settings
impl From<IniSettings> for Settings {
    fn from(ini_settings: IniSettings) -> Self {
        let mut settings = Settings::default();

        // COMMON SECTION
        // Process in the same order as the C++ readConf function
        settings.api_mode = ini_settings.api_mode;
        settings.api_access_token = ini_settings.api_access_token;
        settings.default_urls = if !ini_settings.default_url.is_empty() {
            ini_settings
                .default_url
                .split('|')
                .map(String::from)
                .collect()
        } else {
            Vec::new()
        };
        settings.enable_insert = ini_settings.enable_insert;
        settings.insert_urls = if !ini_settings.insert_url.is_empty() {
            ini_settings
                .insert_url
                .split('|')
                .map(String::from)
                .collect()
        } else {
            Vec::new()
        };
        settings.prepend_insert = ini_settings.prepend_insert_url;
        settings.exclude_remarks = ini_settings.exclude_remarks;
        settings.include_remarks = ini_settings.include_remarks;
        settings.filter_script = ini_settings.filter_script.clone();
        settings.enable_filter = ini_settings.enable_filter;
        settings.base_path = ini_settings.base_path.clone();
        settings.clash_base = ini_settings.clash_base.clone();
        settings.surge_base = ini_settings.surge_base.clone();
        settings.surfboard_base = ini_settings.surfboard_base.clone();
        settings.mellow_base = ini_settings.mellow_base.clone();
        settings.quan_base = ini_settings.quan_base.clone();
        settings.quanx_base = ini_settings.quanx_base.clone();
        settings.loon_base = ini_settings.loon_base.clone();
        settings.ssub_base = ini_settings.ssub_base.clone();
        settings.singbox_base = ini_settings.singbox_base.clone();
        settings.default_ext_config = ini_settings.default_ext_config.clone();
        settings.append_type = ini_settings.append_type;
        settings.proxy_config = ini_settings.proxy_config.clone();
        settings.proxy_ruleset = ini_settings.proxy_ruleset.clone();
        settings.proxy_subscription = ini_settings.proxy_subscription.clone();
        settings.reload_conf_on_request = ini_settings.reload_conf_on_request;

        // SURGE EXTERNAL PROXY SECTION
        settings.surge_ssr_path = ini_settings.surge_ssr_path.clone();
        settings.surge_resolve_hostname = ini_settings.surge_resolve_hostname;

        // NODE PREFERENCES SECTION
        settings.udp_flag = ini_settings.udp_flag;
        settings.tfo_flag = ini_settings.tfo_flag;
        settings.skip_cert_verify = ini_settings.skip_cert_verify;
        settings.tls13_flag = ini_settings.tls13_flag;
        settings.enable_sort = ini_settings.enable_sort;
        settings.sort_script = ini_settings.sort_script.clone();
        settings.filter_deprecated = ini_settings.filter_deprecated;
        settings.append_userinfo = ini_settings.append_sub_userinfo;
        settings.clash_use_new_field = ini_settings.clash_use_new_field;
        settings.clash_proxies_style = ini_settings.clash_proxies_style.clone();
        settings.clash_proxy_groups_style = ini_settings.clash_proxy_groups_style.clone();
        settings.singbox_add_clash_modes = ini_settings.singbox_add_clash_modes;
        // Set rename_node from parsed_rename
        settings.renames = ini_settings.parsed_rename;

        // USERINFO SECTION
        // Set stream_rule and time_rule from parsed values
        settings.stream_rules = ini_settings.parsed_stream_rule;
        settings.time_rules = ini_settings.parsed_time_rule;

        // MANAGED CONFIG SECTION
        settings.write_managed_config = ini_settings.write_managed_config;
        settings.managed_config_prefix = ini_settings.managed_config_prefix.clone();
        settings.update_interval = ini_settings.update_interval;
        settings.update_strict = ini_settings.update_strict;
        settings.quanx_dev_id = ini_settings.quanx_dev_id.clone();

        // RULESET SECTION
        settings.enable_rule_gen = ini_settings.enable_rule_gen;
        if ini_settings.enable_rule_gen {
            settings.overwrite_original_rules = ini_settings.overwrite_original_rules;
            settings.update_ruleset_on_request = ini_settings.update_ruleset_on_request;
            // Convert string rulesets to RulesetConfig
            settings.custom_rulesets = ini_settings.parsed_ruleset;
        } else {
            settings.overwrite_original_rules = false;
            settings.update_ruleset_on_request = false;
        }
        // PROXY GROUP SECTION
        settings.custom_proxy_groups = ini_settings.parsed_proxy_group;

        // TEMPLATE SECTION
        settings.template_path = ini_settings.template_path.clone();
        settings.template_vars = ini_settings.template_vars.clone();

        // ALIASES SECTION
        settings.aliases = ini_settings.aliases;

        // TASKS SECTION
        settings.enable_cron = ini_settings.enable_cron;
        settings.cron_tasks = ini_settings.parsed_tasks;

        // SERVER SECTION
        settings.listen_address = ini_settings.listen_address.clone();
        settings.listen_port = ini_settings.listen_port;
        settings.serve_file = ini_settings.serve_file;
        settings.serve_file_root = ini_settings.serve_file_root.clone();

        // ADVANCED SECTION
        settings.log_level = ini_settings.log_level;
        if ini_settings.print_dbg_info {
            settings.log_level = LOG_LEVEL_VERBOSE;
        }
        settings.max_pending_conns = ini_settings.max_pending_conns;
        settings.max_concur_threads = ini_settings.max_concur_threads;
        settings.max_allowed_rulesets = ini_settings.max_allowed_rulesets;
        settings.max_allowed_rules = ini_settings.max_allowed_rules;
        settings.max_allowed_download_size = ini_settings.max_allowed_download_size;
        if ini_settings.enable_cache {
            settings.cache_subscription = ini_settings.cache_subscription;
            settings.cache_config = ini_settings.cache_config;
            settings.cache_ruleset = ini_settings.cache_ruleset;
            settings.serve_cache_on_fetch_fail = ini_settings.serve_cache_on_fetch_fail;
        } else {
            settings.cache_subscription = 0;
            settings.cache_config = 0;
            settings.cache_ruleset = 0;
            settings.serve_cache_on_fetch_fail = false;
        }
        settings.script_clean_context = ini_settings.script_clean_context;
        settings.async_fetch_ruleset = ini_settings.async_fetch_ruleset;
        settings.skip_failed_links = ini_settings.skip_failed_links;

        // EMOJIS SECTION
        settings.add_emoji = ini_settings.add_emoji;
        settings.remove_emoji = ini_settings.remove_emoji;
        settings.emojis = ini_settings.parsed_emoji_rules;

        // Ensure listen_address is not empty, as done in the C++ code
        if settings.listen_address.trim().is_empty() {
            settings.listen_address = default_listen_address();
        }

        settings
    }
}
