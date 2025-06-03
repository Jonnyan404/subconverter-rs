use crate::models::cron::{CronTaskConfig, CronTaskConfigs};
use crate::models::proxy_group_config::{ProxyGroupConfig, ProxyGroupConfigs, ProxyGroupType};
use crate::models::regex_match_config::{RegexMatchConfig, RegexMatchConfigs};
use crate::models::ruleset::{RulesetConfig, RulesetConfigs};
use crate::utils::string::starts_with;

/// Parse group times string into interval, timeout, and tolerance values
/// Similar to the C++ parseGroupTimes function in settings.h
fn parse_group_times(src: &str, interval: &mut u32, timeout: &mut u32, tolerance: &mut u32) {
    let parts: Vec<&str> = src.split(',').collect();
    if parts.len() >= 1 {
        if let Ok(val) = parts[0].parse::<u32>() {
            *interval = val;
        }
    }
    if parts.len() >= 2 {
        if let Ok(val) = parts[1].parse::<u32>() {
            *timeout = val;
        }
    }
    if parts.len() >= 3 {
        if let Ok(val) = parts[2].parse::<u32>() {
            *tolerance = val;
        }
    }
}

/// Trait for parsing types from INI string arrays
pub trait FromIni<T> {
    /// Convert from INI string array to the target type
    fn from_ini(arr: &[String]) -> T;
}

/// Trait for parsing types from INI string arrays with a delimiter
pub trait FromIniWithDelimiter<T> {
    /// Convert from INI string array with a custom delimiter to the target type
    fn from_ini_with_delimiter(arr: &[String], delimiter: &str) -> T;
}

/// Implementation for parsing ProxyGroupConfig from INI string lines
impl FromIni<ProxyGroupConfigs> for ProxyGroupConfigs {
    fn from_ini(arr: &[String]) -> ProxyGroupConfigs {
        let mut confs = Vec::new();

        for x in arr {
            let mut rules_upper_bound;
            let mut conf = ProxyGroupConfig::default();

            let v_array: Vec<&str> = x.split('`').collect();
            if v_array.len() < 3 {
                continue;
            }

            conf.name = v_array[0].to_string();
            let type_str = v_array[1];

            rules_upper_bound = v_array.len();
            conf.group_type = match type_str {
                "select" => ProxyGroupType::Select,
                "relay" => ProxyGroupType::Relay,
                "url-test" => ProxyGroupType::URLTest,
                "fallback" => ProxyGroupType::Fallback,
                "load-balance" => ProxyGroupType::LoadBalance,
                "ssid" => ProxyGroupType::SSID,
                "smart" => ProxyGroupType::Smart,
                _ => ProxyGroupType::Select,
            };

            if conf.group_type == ProxyGroupType::URLTest
                || conf.group_type == ProxyGroupType::LoadBalance
                || conf.group_type == ProxyGroupType::Fallback
            {
                if rules_upper_bound < 5 {
                    continue;
                }
                rules_upper_bound -= 2;
                conf.url = v_array[rules_upper_bound].to_string();

                let mut interval = 0;
                let mut timeout = 5;
                let mut tolerance = 0;
                parse_group_times(
                    v_array[rules_upper_bound + 1],
                    &mut interval,
                    &mut timeout,
                    &mut tolerance,
                );
                conf.interval = interval;
                conf.timeout = timeout;
                conf.tolerance = tolerance;
            }

            for i in 2..rules_upper_bound {
                if starts_with(v_array[i], "!!PROVIDER=") {
                    let provider_list: Vec<&str> = v_array[i][11..].split(',').collect();
                    for provider in provider_list {
                        conf.using_provider.push(provider.to_string());
                    }
                } else {
                    conf.proxies.push(v_array[i].to_string());
                }
            }

            confs.push(conf);
        }

        confs
    }
}

/// Implementation for parsing RulesetConfig from INI string lines
impl FromIni<Vec<RulesetConfig>> for RulesetConfigs {
    fn from_ini(arr: &[String]) -> Vec<RulesetConfig> {
        let mut confs = Vec::new();

        for x in arr {
            let mut conf = RulesetConfig::default();

            let pos = x.find(',');
            if pos.is_none() {
                continue;
            }

            let pos = pos.unwrap();
            conf.group = x[..pos].to_string();

            // Handle the case where URL starts with "[]"
            if x.len() > pos + 3 && &x[pos + 1..pos + 3] == "[]" {
                conf.url = x[pos + 1..].to_string();
                confs.push(conf);
                continue;
            }

            // Check if there's an interval specified
            let epos = x.rfind(',');
            if pos != epos.unwrap_or(pos) {
                let epos = epos.unwrap();
                if let Ok(interval) = x[epos + 1..].parse::<u32>() {
                    conf.interval = interval;
                }
                conf.url = x[pos + 1..epos].to_string();
            } else {
                conf.url = x[pos + 1..].to_string();
            }

            confs.push(conf);
        }

        confs
    }
}

/// Implementation for parsing RegexMatchConfig from INI string lines with
/// delimiter
impl FromIniWithDelimiter<RegexMatchConfigs> for RegexMatchConfigs {
    fn from_ini_with_delimiter(arr: &[String], delimiter: &str) -> RegexMatchConfigs {
        let mut confs = Vec::new();

        for x in arr {
            let mut conf = RegexMatchConfig::new(String::new(), String::new(), String::new());

            // Handle script case
            if starts_with(x, "script:") {
                conf.script = x[7..].to_string();
                confs.push(conf);
                continue;
            }

            // Handle match/replace case
            let pos = x.rfind(delimiter);
            conf._match = x[..pos.unwrap_or(x.len())].to_string();

            if let Some(p) = pos {
                if p < x.len() - 1 {
                    conf.replace = x[p + delimiter.len()..].to_string();
                }
            }

            conf.compile();

            confs.push(conf);
        }

        confs
    }
}

/// Implementation for parsing CronTaskConfig from INI string lines
impl FromIni<CronTaskConfigs> for CronTaskConfigs {
    fn from_ini(arr: &[String]) -> CronTaskConfigs {
        let mut confs = Vec::new();

        for x in arr {
            let mut conf = CronTaskConfig::default();

            let v_array: Vec<&str> = x.split('`').collect();
            if v_array.len() < 3 {
                continue;
            }

            conf.name = v_array[0].to_string();
            conf.cron_exp = v_array[1].to_string();
            conf.path = v_array[2].to_string();

            if v_array.len() > 3 {
                if let Ok(timeout) = v_array[3].parse::<u32>() {
                    conf.timeout = timeout;
                }
            }

            confs.push(conf);
        }

        confs
    }
}
