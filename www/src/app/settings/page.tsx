"use client";

import { useState, useEffect } from 'react';
import { readSettingsFile, writeSettingsFile } from '@/lib/api-client';
import { useTranslations } from 'next-intl';
import Link from 'next/link';
import yaml from 'js-yaml';
import CodeEditor from '@/components/CodeEditor';

// Settings interface based on pref.yml structure
interface SubconverterSettings {
    common?: {
        api_mode?: boolean;
        api_access_token?: string;
        default_url?: string[];
        enable_insert?: boolean;
        insert_url?: string[];
        prepend_insert_url?: boolean;
        exclude_remarks?: string[];
        include_remarks?: string[];
        enable_filter?: boolean;
        filter_script?: string;
        default_external_config?: string;
        base_path?: string;
        clash_rule_base?: string;
        surge_rule_base?: string;
        surfboard_rule_base?: string;
        mellow_rule_base?: string;
        quan_rule_base?: string;
        quanx_rule_base?: string;
        loon_rule_base?: string;
        sssub_rule_base?: string;
        singbox_rule_base?: string;
        proxy_config?: string;
        proxy_ruleset?: string;
        proxy_subscription?: string;
        append_proxy_type?: boolean;
        reload_conf_on_request?: boolean;
    };
    userinfo?: {
        stream_rule?: Array<{ match: string; replace: string }>;
        time_rule?: Array<{ match: string; replace: string }>;
    };
    node_pref?: {
        udp_flag?: boolean;
        tcp_fast_open_flag?: boolean;
        skip_cert_verify_flag?: boolean;
        tls13_flag?: boolean;
        sort_flag?: boolean;
        sort_script?: string;
        filter_deprecated_nodes?: boolean;
        append_sub_userinfo?: boolean;
        clash_use_new_field_name?: boolean;
        clash_proxies_style?: string;
        clash_proxy_groups_style?: string;
        singbox_add_clash_modes?: boolean;
        rename_node?: Array<{ match?: string; replace?: string; script?: string; import?: string }>;
    };
    managed_config?: {
        write_managed_config?: boolean;
        managed_config_prefix?: string;
        config_update_interval?: number;
        config_update_strict?: boolean;
        quanx_device_id?: string;
    };
    surge_external_proxy?: {
        surge_ssr_path?: string;
        resolve_hostname?: boolean;
    };
    emojis?: {
        add_emoji?: boolean;
        remove_old_emoji?: boolean;
        rules?: Array<{ match?: string; emoji?: string; script?: string; import?: string }>;
    };
    rulesets?: {
        enabled?: boolean;
        overwrite_original_rules?: boolean;
        update_ruleset_on_request?: boolean;
        rulesets?: Array<{ rule?: string; ruleset?: string; group?: string; interval?: number; import?: string }>;
    };
    proxy_groups?: {
        custom_proxy_group?: Array<{ name?: string; type?: string; rule?: string[]; url?: string; interval?: number; tolerance?: number; timeout?: number; import?: string }>;
    };
    template?: {
        template_path?: string;
        globals?: Array<{ key: string; value: any }>;
    };
    aliases?: Array<{ uri: string; target: string }>;
    tasks?: Array<{ name: string; cronexp: string; path: string; timeout?: number }>;
    server?: {
        listen?: string;
        port?: number;
        serve_file_root?: string;
    };
    advanced?: {
        log_level?: string;
        print_debug_info?: boolean;
        max_pending_connections?: number;
        max_concurrent_threads?: number;
        max_allowed_rulesets?: number;
        max_allowed_rules?: number;
        max_allowed_download_size?: number;
        enable_cache?: boolean;
        cache_subscription?: number;
        cache_config?: number;
        cache_ruleset?: number;
        script_clean_context?: boolean;
        async_fetch_ruleset?: boolean;
        skip_failed_links?: boolean;
    };
}

export default function SettingsPage() {
    const t = useTranslations('SettingsPage');
    const commonT = useTranslations('Common');

    const [settings, setSettings] = useState<SubconverterSettings>({});
    const [originalYaml, setOriginalYaml] = useState<string>('');
    const [isLoading, setIsLoading] = useState(true);
    const [isSaving, setIsSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [saveSuccess, setSaveSuccess] = useState(false);
    const [activeTab, setActiveTab] = useState('common');
    const [yamlPreviewContent, setYamlPreviewContent] = useState('');

    // Add state to track unsaved changes in CodeEditors
    const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

    useEffect(() => {
        loadSettings();
    }, []);

    useEffect(() => {
        if (!isLoading) {
            try {
                const currentYaml = yaml.dump(settings, {
                    indent: 2,
                    lineWidth: -1,
                    noRefs: true,
                    sortKeys: false
                });
                setYamlPreviewContent(currentYaml);
            } catch (err) {
                console.error("Error generating YAML preview:", err);
                setYamlPreviewContent("# Error generating YAML preview");
            }
        }
    }, [settings, isLoading]);

    const loadSettings = async () => {
        setIsLoading(true);
        setError(null);
        try {
            const yamlContent = await readSettingsFile();
            setOriginalYaml(yamlContent);

            const parsedSettings = yaml.load(yamlContent) as SubconverterSettings;
            setSettings(parsedSettings || {});
        } catch (err) {
            setError(t('loadError', { message: err instanceof Error ? err.message : String(err) }));
            console.error("Error loading settings:", err);
        } finally {
            setIsLoading(false);
        }
    };

    const saveSettings = async () => {
        setIsSaving(true);
        setSaveSuccess(false);
        setError(null);

        try {
            const yamlContent = yaml.dump(settings, {
                indent: 2,
                lineWidth: -1, // Don't wrap lines
                noRefs: true,
                sortKeys: false // Preserve key order
            });

            await writeSettingsFile(yamlContent);
            setOriginalYaml(yamlContent);
            setSaveSuccess(true);

            // Hide success message after 3 seconds
            setTimeout(() => setSaveSuccess(false), 3000);
        } catch (err) {
            setError(t('saveError', { message: err instanceof Error ? err.message : String(err) }));
            console.error("Error saving settings:", err);
        } finally {
            setIsSaving(false);
        }
    };

    const handleInputChange = (section: keyof SubconverterSettings, key: string, value: any) => {
        setSettings(prevSettings => ({
            ...prevSettings,
            [section]: {
                ...prevSettings[section],
                [key]: value
            }
        }));
    };

    // Handle array input changes (like exclude_remarks)
    const handleArrayChange = (section: keyof SubconverterSettings, key: string, value: string) => {
        const arrayValue = value.split(',').map(item => item.trim());
        setSettings(prevSettings => ({
            ...prevSettings,
            [section]: {
                ...prevSettings[section],
                [key]: arrayValue
            }
        }));
    };

    // Callback for CodeEditor save
    const handleCodeEditorSave = () => {
        setHasUnsavedChanges(false); // Reset unsaved changes flag on successful save
        // Optionally: show a success message specific to the editor
    };

    // Callback for CodeEditor change
    const handleCodeEditorChange = () => {
        setHasUnsavedChanges(true); // Set unsaved changes flag
    };

    const renderCommonSection = () => {
        const common = settings.common || {};
        return (
            <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">{t('common.apiMode')}</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={common.api_mode ? "true" : "false"}
                            onChange={(e) => handleInputChange('common', 'api_mode', e.target.value === "true")}
                        >
                            <option value="true">{commonT('enabled')}</option>
                            <option value="false">{commonT('disabled')}</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">{t('common.apiAccessToken')}</label>
                        <input
                            type="text"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={common?.api_access_token || ''}
                            onChange={(e) => handleInputChange('common', 'api_access_token', e.target.value)}
                        />
                    </div>
                </div>

                <div>
                    <label className="block text-sm font-medium mb-1">{t('common.defaultUrl')}</label>
                    <input
                        type="text"
                        className="w-full p-2 border border-gray-300 rounded bg-white/10"
                        value={(common?.default_url || []).join(', ')}
                        onChange={(e) => handleArrayChange('common', 'default_url', e.target.value)}
                    />
                    <p className="mt-1 text-xs text-gray-400">{t('common.defaultUrlHelp')}</p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">{t('common.enableInsert')}</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={common?.enable_insert ? "true" : "false"}
                            onChange={(e) => handleInputChange('common', 'enable_insert', e.target.value === "true")}
                        >
                            <option value="true">{commonT('enabled')}</option>
                            <option value="false">{commonT('disabled')}</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">{t('common.prependInsertUrl')}</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={common?.prepend_insert_url ? "true" : "false"}
                            onChange={(e) => handleInputChange('common', 'prepend_insert_url', e.target.value === "true")}
                        >
                            <option value="true">{commonT('enabled')}</option>
                            <option value="false">{commonT('disabled')}</option>
                        </select>
                    </div>
                </div>

                <div>
                    <label className="block text-sm font-medium mb-1">{t('common.insertUrl')}</label>
                    <input
                        type="text"
                        className="w-full p-2 border border-gray-300 rounded bg-white/10"
                        value={(common?.insert_url || []).join(', ')}
                        onChange={(e) => handleArrayChange('common', 'insert_url', e.target.value)}
                    />
                    <p className="mt-1 text-xs text-gray-400">{t('common.insertUrlHelp')}</p>
                </div>
            </div>
        );
    };

    const renderServerSection = () => {
        const server = settings.server || {};
        return (
            <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Listen Address</label>
                        <input
                            type="text"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={server?.listen || ''}
                            onChange={(e) => handleInputChange('server', 'listen', e.target.value)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Port</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={server?.port || ''}
                            onChange={(e) => handleInputChange('server', 'port', parseInt(e.target.value) || 0)}
                        />
                    </div>
                </div>

                <div>
                    <label className="block text-sm font-medium mb-1">Serve File Root</label>
                    <input
                        type="text"
                        className="w-full p-2 border border-gray-300 rounded bg-white/10"
                        value={server?.serve_file_root || ''}
                        onChange={(e) => handleInputChange('server', 'serve_file_root', e.target.value)}
                    />
                </div>
            </div>
        );
    };

    const renderAdvancedSection = () => {
        const advanced = settings.advanced || {};
        return (
            <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Log Level</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.log_level || 'info'}
                            onChange={(e) => handleInputChange('advanced', 'log_level', e.target.value)}
                        >
                            <option value="debug">Debug</option>
                            <option value="info">Info</option>
                            <option value="warn">Warning</option>
                            <option value="error">Error</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Print Debug Info</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.print_debug_info ? "true" : "false"}
                            onChange={(e) => handleInputChange('advanced', 'print_debug_info', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Max Pending Connections</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.max_pending_connections || ''}
                            onChange={(e) => handleInputChange('advanced', 'max_pending_connections', parseInt(e.target.value) || 0)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Max Concurrent Threads</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.max_concurrent_threads || ''}
                            onChange={(e) => handleInputChange('advanced', 'max_concurrent_threads', parseInt(e.target.value) || 0)}
                        />
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Max Allowed Rulesets</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.max_allowed_rulesets || ''}
                            onChange={(e) => handleInputChange('advanced', 'max_allowed_rulesets', parseInt(e.target.value) || 0)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Max Allowed Rules</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.max_allowed_rules || ''}
                            onChange={(e) => handleInputChange('advanced', 'max_allowed_rules', parseInt(e.target.value) || 0)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Max Download Size (bytes)</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.max_allowed_download_size || ''}
                            onChange={(e) => handleInputChange('advanced', 'max_allowed_download_size', parseInt(e.target.value) || 0)}
                        />
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Enable Cache</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.enable_cache ? "true" : "false"}
                            onChange={(e) => handleInputChange('advanced', 'enable_cache', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Skip Failed Links</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.skip_failed_links ? "true" : "false"}
                            onChange={(e) => handleInputChange('advanced', 'skip_failed_links', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Cache Subscription (seconds)</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.cache_subscription || ''}
                            onChange={(e) => handleInputChange('advanced', 'cache_subscription', parseInt(e.target.value) || 0)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Cache Config (seconds)</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.cache_config || ''}
                            onChange={(e) => handleInputChange('advanced', 'cache_config', parseInt(e.target.value) || 0)}
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Cache Ruleset (seconds)</label>
                        <input
                            type="number"
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={advanced?.cache_ruleset || ''}
                            onChange={(e) => handleInputChange('advanced', 'cache_ruleset', parseInt(e.target.value) || 0)}
                        />
                    </div>
                </div>
            </div>
        );
    };

    const renderNodePrefSection = () => {
        const nodePref = settings.node_pref || {};
        return (
            <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Enable UDP</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.udp_flag === undefined ? "" : (nodePref?.udp_flag ? "true" : "false")}
                            onChange={(e) => {
                                if (e.target.value === "") {
                                    const newSettings = { ...settings };
                                    if (newSettings.node_pref) {
                                        delete newSettings.node_pref.udp_flag;
                                        setSettings(newSettings);
                                    }
                                } else {
                                    handleInputChange('node_pref', 'udp_flag', e.target.value === "true");
                                }
                            }}
                        >
                            <option value="">Not Set</option>
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">TFO (TCP Fast Open)</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.tcp_fast_open_flag === undefined ? "" : (nodePref?.tcp_fast_open_flag ? "true" : "false")}
                            onChange={(e) => {
                                if (e.target.value === "") {
                                    const newSettings = { ...settings };
                                    if (newSettings.node_pref) {
                                        delete newSettings.node_pref.tcp_fast_open_flag;
                                        setSettings(newSettings);
                                    }
                                } else {
                                    handleInputChange('node_pref', 'tcp_fast_open_flag', e.target.value === "true");
                                }
                            }}
                        >
                            <option value="">Not Set</option>
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Skip Cert Verify</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.skip_cert_verify_flag === undefined ? "" : (nodePref?.skip_cert_verify_flag ? "true" : "false")}
                            onChange={(e) => {
                                if (e.target.value === "") {
                                    const newSettings = { ...settings };
                                    if (newSettings.node_pref) {
                                        delete newSettings.node_pref.skip_cert_verify_flag;
                                        setSettings(newSettings);
                                    }
                                } else {
                                    handleInputChange('node_pref', 'skip_cert_verify_flag', e.target.value === "true");
                                }
                            }}
                        >
                            <option value="">Not Set</option>
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Enable TLS 1.3</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.tls13_flag === undefined ? "" : (nodePref?.tls13_flag ? "true" : "false")}
                            onChange={(e) => {
                                if (e.target.value === "") {
                                    const newSettings = { ...settings };
                                    if (newSettings.node_pref) {
                                        delete newSettings.node_pref.tls13_flag;
                                        setSettings(newSettings);
                                    }
                                } else {
                                    handleInputChange('node_pref', 'tls13_flag', e.target.value === "true");
                                }
                            }}
                        >
                            <option value="">Not Set</option>
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Sort Nodes</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.sort_flag ? "true" : "false"}
                            onChange={(e) => handleInputChange('node_pref', 'sort_flag', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Filter Deprecated Nodes</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.filter_deprecated_nodes ? "true" : "false"}
                            onChange={(e) => handleInputChange('node_pref', 'filter_deprecated_nodes', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Append Sub User Info</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.append_sub_userinfo ? "true" : "false"}
                            onChange={(e) => handleInputChange('node_pref', 'append_sub_userinfo', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Clash Use New Field Names</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.clash_use_new_field_name ? "true" : "false"}
                            onChange={(e) => handleInputChange('node_pref', 'clash_use_new_field_name', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">SingBox Add Clash Modes</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.singbox_add_clash_modes ? "true" : "false"}
                            onChange={(e) => handleInputChange('node_pref', 'singbox_add_clash_modes', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Clash Proxies Style</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.clash_proxies_style || 'flow'}
                            onChange={(e) => handleInputChange('node_pref', 'clash_proxies_style', e.target.value)}
                        >
                            <option value="flow">Flow</option>
                            <option value="block">Block</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Clash Proxy Groups Style</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={nodePref?.clash_proxy_groups_style || 'flow'}
                            onChange={(e) => handleInputChange('node_pref', 'clash_proxy_groups_style', e.target.value)}
                        >
                            <option value="flow">Flow</option>
                            <option value="block">Block</option>
                        </select>
                    </div>
                </div>
            </div>
        );
    };

    const renderEmojisSection = () => {
        const emojis = settings.emojis || {};
        return (
            <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm font-medium mb-1">Add Emoji</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={emojis?.add_emoji ? "true" : "false"}
                            onChange={(e) => handleInputChange('emojis', 'add_emoji', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>

                    <div>
                        <label className="block text-sm font-medium mb-1">Remove Old Emoji</label>
                        <select
                            className="w-full p-2 border border-gray-300 rounded bg-white/10"
                            value={emojis?.remove_old_emoji ? "true" : "false"}
                            onChange={(e) => handleInputChange('emojis', 'remove_old_emoji', e.target.value === "true")}
                        >
                            <option value="true">Enabled</option>
                            <option value="false">Disabled</option>
                        </select>
                    </div>
                </div>

                <div>
                    <label className="block text-sm font-medium mb-1">Emoji Rules</label>
                    <p className="text-xs mb-2 text-gray-400">
                        Emoji rules are defined in snippets/emoji.txt by default. You can modify that file directly.
                    </p>
                </div>
            </div>
        );
    };

    const renderSnippetsSection = () => { // New function for snippets tab
        return (
            <div className="space-y-6">
                <div>
                    <h3 className="text-lg font-semibold mb-2">emoji.txt</h3>
                    <p className="text-sm text-gray-400 mb-2">{t('snippets.emojiFileDesc')}</p>
                    <div className="bg-gray-800 rounded-lg shadow-md overflow-hidden h-96">
                        <CodeEditor
                            filePath="snippets/emoji.txt"
                            language="plaintext"
                            onSave={handleCodeEditorSave}
                            onChange={handleCodeEditorChange}
                        />
                    </div>
                </div>
                <div>
                    <h3 className="text-lg font-semibold mb-2">gistconf.ini</h3>
                    <p className="text-sm text-gray-400 mb-2">{t('snippets.gistConfDesc')}</p>
                    <div className="bg-gray-800 rounded-lg shadow-md overflow-hidden h-96">
                        <CodeEditor
                            filePath="gistconf.ini"
                            language="ini"
                            onSave={handleCodeEditorSave}
                            onChange={handleCodeEditorChange}
                        />
                    </div>
                </div>
            </div>
        );
    };

    const renderTab = () => {
        switch (activeTab) {
            case 'common':
                return renderCommonSection();
            case 'server':
                return renderServerSection();
            case 'advanced':
                return renderAdvancedSection();
            case 'node_pref':
                return renderNodePrefSection();
            case 'emojis':
                return renderEmojisSection();
            case 'snippets': // Add case for snippets
                return renderSnippetsSection();
            default:
                return <p>Select a section to edit settings.</p>;
        }
    };

    if (isLoading) {
        return (
            <div className="flex min-h-screen flex-col items-center justify-center">
                <div className="mb-4 text-xl">{t('loading')}</div>
            </div>
        );
    }

    return (
        <main className="flex min-h-screen flex-col items-center justify-between p-4 md:p-8 lg:p-24">
            <div className="z-10 max-w-5xl w-full items-center justify-between font-mono text-sm">
                <div className="flex flex-col sm:flex-row items-center justify-between mb-6">
                    <h1 className="text-4xl font-bold mb-4 sm:mb-0 text-center">{t('title')}</h1>
                    <Link
                        href="/"
                        className="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                    >
                        {t('backToHome')}
                    </Link>
                </div>

                {error && (
                    <div className="bg-red-100 border-l-4 border-red-500 text-red-700 p-4 mb-6">
                        <p>{error}</p>
                    </div>
                )}

                {saveSuccess && (
                    <div className="bg-green-100 border-l-4 border-green-500 text-green-700 p-4 mb-6">
                        <p>{t('saveSuccess')}</p>
                    </div>
                )}

                <div className="bg-white/5 rounded-lg shadow-md overflow-hidden">
                    <div className="flex flex-wrap border-b border-gray-300">
                        {/* Pref.yml related tabs */}
                        <button
                            className={`px-4 py-2 ${activeTab === 'common' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('common')}
                        >
                            {t('tabs.common')}
                        </button>
                        <button
                            className={`px-4 py-2 ${activeTab === 'server' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('server')}
                        >
                            {t('tabs.server')}
                        </button>
                        <button
                            className={`px-4 py-2 ${activeTab === 'advanced' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('advanced')}
                        >
                            {t('tabs.advanced')}
                        </button>
                        <button
                            className={`px-4 py-2 ${activeTab === 'node_pref' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('node_pref')}
                        >
                            {t('tabs.node_pref')}
                        </button>
                        <button
                            className={`px-4 py-2 ${activeTab === 'emojis' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('emojis')}
                        >
                            {t('tabs.emojis')}
                        </button>

                        {/* Separator (Optional visual cue) */}
                        <div className="border-l border-gray-400 mx-2"></div>

                        {/* Direct file editing tabs */}
                        <button // Add Snippets tab button
                            className={`px-4 py-2 ${activeTab === 'snippets' ? 'bg-blue-600 text-white' : 'bg-gray-200 text-gray-800'}`}
                            onClick={() => setActiveTab('snippets')}
                        >
                            {t('snippets.title')}
                        </button>
                    </div>

                    <div className="p-6">
                        {renderTab()}

                        {/* Only show Save button for settings tabs, not snippets */}
                        {activeTab !== 'snippets' && (
                            <div className="mt-6 flex justify-end">
                                <button
                                    className="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded disabled:opacity-50"
                                    onClick={saveSettings}
                                    disabled={isSaving}
                                >
                                    {isSaving ? t('savingSettings') : t('saveButton')}
                                </button>
                            </div>
                        )}
                    </div>
                </div>

                {/* Conditionally render YAML Preview */}
                {activeTab !== 'snippets' && (
                    <div className="mt-8">
                        <h2 className="text-2xl font-semibold mb-4">YAML Preview (Read-only)</h2>
                        <div className="bg-gray-800 rounded-lg shadow-md overflow-hidden h-96"> {/* Set fixed height */}
                            <CodeEditor
                                filePath="pref.yaml (Preview)"
                                language="yaml"
                                value={yamlPreviewContent}
                                options={{ readOnly: true }}
                            />
                        </div>
                        <p className="mt-2 text-xs text-gray-400">
                            This is a preview of the <code className="text-xs bg-gray-700 px-1 rounded">pref.yml</code> file based on your current settings. Changes here won't be saved directly; use the form above.
                        </p>
                    </div>
                )}
            </div>
        </main>
    );
}