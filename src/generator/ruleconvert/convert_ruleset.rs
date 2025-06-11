//! Rule conversion implementation between different proxy configuration formats
//!
//! Converts proxy rule formats between Clash, Surge, and Quantumult X

use crate::models::RulesetType;
use crate::utils::network::is_ipv4;
use crate::utils::string::ends_with;
use regex::Regex;

/// Converts a ruleset from one format to another
///
/// # Arguments
///
/// * `content` - The ruleset content to convert
/// * `ruleset_type` - The target ruleset type
///
/// # Returns
///
/// The converted ruleset content
pub fn convert_ruleset(content: &str, ruleset_type: RulesetType) -> String {
    // If target type is Surge, return content as is
    if ruleset_type == RulesetType::Surge {
        return content.to_string();
    }

    let mut output = String::new();
    let payload_regex = Regex::new(r"^payload:\r?\n").unwrap();

    if payload_regex.is_match(content) {
        // Convert Clash ruleset to Surge format

        // First, replace the payload header and format the rules
        let content_without_header = content.replace("payload:", "").trim().to_string();

        // Process each line to extract rule content
        let mut rule_items_formatted = String::new();
        for line in content_without_header.lines() {
            let line = line.trim();
            if line.starts_with('-') {
                // Extract the actual rule content, removing the dash and quotes
                let mut rule_content = line[1..].trim().to_string();
                if rule_content.starts_with('\'') && rule_content.ends_with('\'') {
                    rule_content = rule_content[1..rule_content.len() - 1].to_string();
                } else if rule_content.starts_with('"') && rule_content.ends_with('"') {
                    rule_content = rule_content[1..rule_content.len() - 1].to_string();
                }
                rule_items_formatted.push_str(&rule_content);
                rule_items_formatted.push('\n');
            }
        }

        // If target is Clash Classical, return the formatted rules
        if ruleset_type == RulesetType::ClashClassical {
            return rule_items_formatted;
        }

        // Process each line and convert to appropriate format
        for line in rule_items_formatted.lines() {
            let mut line = line.trim().to_string();

            // Remove trailing \r if present
            if line.ends_with('\r') {
                line.pop();
            }

            // Remove comments
            if let Some(comment_pos) = line.find("//") {
                line = line[..comment_pos].trim().to_string();
            }

            // Skip empty lines and comments
            if line.is_empty()
                || line.starts_with(';')
                || line.starts_with('#')
                || (line.len() >= 2 && line.starts_with("//"))
            {
                continue;
            }

            // Process actual rules
            if let Some(pos) = line.find('/') {
                // IP-CIDR or IP-CIDR6 classification
                if is_ipv4(&line[..pos]) {
                    output.push_str("IP-CIDR,");
                } else {
                    output.push_str("IP-CIDR6,");
                }
                output.push_str(&line);
            } else if line.starts_with('.') || (line.len() >= 2 && line.starts_with("+.")) {
                // Domain suffix or keyword
                let mut keyword_flag = false;
                let mut rule_content = line.clone();

                // Check for keyword pattern (ends with .*)
                while ends_with(&rule_content, ".*") {
                    keyword_flag = true;
                    rule_content = rule_content[..rule_content.len() - 2].to_string();
                }

                output.push_str("DOMAIN-");
                if keyword_flag {
                    output.push_str("KEYWORD,");
                } else {
                    output.push_str("SUFFIX,");
                }

                // Remove leading dot or "+."
                if rule_content.starts_with("+.") {
                    rule_content = rule_content[2..].to_string();
                } else if rule_content.starts_with('.') {
                    rule_content = rule_content[1..].to_string();
                }

                output.push_str(&rule_content);
            } else {
                // Plain domain
                output.push_str("DOMAIN,");
                output.push_str(&line);
            }

            output.push('\n');
        }
    } else {
        // Convert Quantumult X ruleset to Surge format

        // Replace HOST with DOMAIN and IP6-CIDR with IP-CIDR6
        let host_regex = Regex::new(r"(?i)^host").unwrap();
        let ip6_cidr_regex = Regex::new(r"(?i)^ip6-cidr").unwrap();

        let mut processed = host_regex.replace_all(content, "DOMAIN").to_string();
        processed = ip6_cidr_regex
            .replace_all(&processed, "IP-CIDR6")
            .to_string();

        // Remove group info and standardize format
        // This regex matches the rule type and pattern, removes any group, and preserves no-resolve if present
        let rule_format_regex = Regex::new(
            r"^((?i:DOMAIN(?:-(?:SUFFIX|KEYWORD))?|IP-CIDR6?|USER-AGENT),)\s*?(\S*?)(?:,(?!no-resolve).*?)(,no-resolve)?$"
        ).unwrap();

        output = rule_format_regex
            .replace_all(&processed, "$1$2$3")
            .to_string();

        // Convert rule types to uppercase
        let domain_regex = Regex::new(r"^(domain)").unwrap();
        let domain_suffix_regex = Regex::new(r"^(domain-suffix)").unwrap();
        let domain_keyword_regex = Regex::new(r"^(domain-keyword)").unwrap();
        let ip_cidr_regex = Regex::new(r"^(ip-cidr)").unwrap();
        let ip_cidr6_regex = Regex::new(r"^(ip-cidr6)").unwrap();
        let user_agent_regex = Regex::new(r"^(user-agent)").unwrap();

        output = domain_regex.replace_all(&output, "DOMAIN").to_string();
        output = domain_suffix_regex
            .replace_all(&output, "DOMAIN-SUFFIX")
            .to_string();
        output = domain_keyword_regex
            .replace_all(&output, "DOMAIN-KEYWORD")
            .to_string();
        output = ip_cidr_regex.replace_all(&output, "IP-CIDR").to_string();
        output = ip_cidr6_regex.replace_all(&output, "IP-CIDR6").to_string();
        output = user_agent_regex
            .replace_all(&output, "USER-AGENT")
            .to_string();
    }

    output
}
