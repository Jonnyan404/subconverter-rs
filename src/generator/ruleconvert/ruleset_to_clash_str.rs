//! Ruleset to Clash string conversion
//!
//! This module provides functionality to convert rulesets to Clash YAML string format.

use crate::models::RulesetContent;
use crate::utils::string::{find_str, starts_with, trim};
use crate::Settings;
use lazy_static::lazy_static;
use log::warn;
use serde_yaml::Value as YamlValue;
use std::collections::HashSet;

use super::common::transform_rule_to_common;
use super::convert_ruleset::convert_ruleset;

lazy_static! {
    static ref CLASH_RULE_TYPES: HashSet<&'static str> = {
        let mut types = HashSet::new();
        // Basic types
        types.insert("DOMAIN");
        types.insert("DOMAIN-SUFFIX");
        types.insert("DOMAIN-KEYWORD");
        types.insert("IP-CIDR");
        types.insert("SRC-IP-CIDR");
        types.insert("GEOIP");
        types.insert("MATCH");
        types.insert("FINAL");
        // Clash-specific types
        types.insert("IP-CIDR6");
        types.insert("SRC-PORT");
        types.insert("DST-PORT");
        types.insert("PROCESS-NAME");
        types
    };
}

/// Converts rulesets to Clash YAML string format
///
/// # Arguments
///
/// * `base_rule` - YAML node containing base rules
/// * `ruleset_content_array` - Array of ruleset content
/// * `overwrite_original_rules` - Whether to overwrite original rules
/// * `new_field_name` - Whether to use "rules" or "Rule" as the field name
///
/// # Returns
///
/// YAML string containing the converted rules
pub fn ruleset_to_clash_str(
    base_rule: &YamlValue,
    ruleset_content_array: &[RulesetContent],
    overwrite_original_rules: bool,
    new_field_name: bool,
) -> String {
    // Get global settings
    let settings = Settings::current();

    // Set field name based on parameter
    let field_name = if new_field_name { "rules" } else { "Rule" };
    let mut output_content = format!("\n{}:\n", field_name);
    let mut total_rules = 0;

    // Include existing rules if not overwriting
    if !overwrite_original_rules {
        if let Some(rules) = base_rule.get(field_name) {
            if let Some(rules_array) = rules.as_sequence() {
                for rule in rules_array {
                    if let Some(rule_str) = rule.as_str() {
                        output_content.push_str(&format!("  - {}\n", rule_str));
                    }
                }
            }
        }
    }

    // Use the max_allowed_rules from global settings
    let max_allowed_rules = settings.max_allowed_rules;

    // Process each ruleset content
    for ruleset in ruleset_content_array {
        // Check if we've reached the maximum number of rules
        if max_allowed_rules > 0 && total_rules >= max_allowed_rules {
            break;
        }

        // Get group name from ruleset
        let rule_group = &ruleset.group;

        // Get rule content from the shared future-like container
        let retrieved_rules = ruleset.get_rule_content();

        // Skip empty rulesets
        if retrieved_rules.is_empty() {
            warn!(
                "Failed to fetch ruleset or ruleset is empty: '{}'!",
                ruleset.rule_path
            );
            continue;
        }

        // Handle special case for rules that start with "[]"
        if starts_with(&retrieved_rules, "[]") {
            let mut rule_line = retrieved_rules[2..].to_string();

            // Replace FINAL with MATCH for Clash compatibility
            if starts_with(&rule_line, "FINAL") {
                rule_line = rule_line.replacen("FINAL", "MATCH", 1);
            }

            // Transform rule to common format
            let transformed = transform_rule_to_common(&rule_line, rule_group, false);
            output_content.push_str(&format!("  - {}\n", transformed));
            total_rules += 1;
            continue;
        }

        // Convert ruleset based on its type
        let processed_rules = convert_ruleset(&retrieved_rules, ruleset.rule_type);

        // Detect line break style
        let _line_break = if processed_rules.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };

        // Process each line in the ruleset
        for line in processed_rules.lines() {
            // Check if we've reached the maximum number of rules
            if max_allowed_rules > 0 && total_rules >= max_allowed_rules {
                break;
            }

            // Trim whitespace from line
            let mut str_line = line.trim().to_string();
            let line_size = str_line.len();

            // Skip empty lines and comments (';', '#', or '//')
            if line_size == 0
                || (line_size >= 1 && (str_line.starts_with(';') || str_line.starts_with('#')))
                || (line_size >= 2 && str_line.starts_with("//"))
            {
                continue;
            }

            // Check if the rule type is supported by Clash
            if !CLASH_RULE_TYPES
                .iter()
                .any(|&rule_type| starts_with(&str_line, rule_type))
            {
                continue;
            }

            // Remove inline comments
            if let Some(comment_pos) = find_str(&str_line, "//") {
                str_line = str_line[..comment_pos].to_string();
                str_line = trim(&str_line).to_string();
            }

            // Transform rule to common format and add to output
            let transformed = transform_rule_to_common(&str_line, rule_group, false);
            output_content.push_str(&format!("  - {}\n", transformed));
            total_rules += 1;
        }
    }

    output_content
}
