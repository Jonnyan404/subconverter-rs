use log::{debug, info};
use std::cmp::Ordering;

use crate::models::{
    extra_settings::ExtraSettings,
    proxy::{Proxy, ProxyType},
    regex_match_config::RegexMatchConfigs,
};
use crate::utils::{
    matcher::{apply_matcher, reg_find},
    reg_replace,
    string::{remove_emoji, trim},
};

use super::matcher::apply_compiled_rule;

/// Applies a rename configuration to a node
/// Similar to the C++ nodeRename function
async fn node_rename(node: &mut Proxy, extra: &mut ExtraSettings) {
    extra.init_js_context();
    let rename_array = &extra.rename_array;
    let original_remark = node.remark.clone();
    for pattern in rename_array {
        if !pattern.script.is_empty() {
            match extra
                .eval_get_rename_node_remark(node, pattern.script.clone())
                .await
            {
                Ok(new_remark) => {
                    node.remark = new_remark;
                }
                Err(e) => {
                    log::error!("Error renaming node: {}", e);
                }
            }
        } else if !pattern._match.is_empty() {
            let mut real_rule = String::new();
            if apply_matcher(&pattern._match, &mut real_rule, node) && !real_rule.is_empty() {
                node.remark = reg_replace(&node.remark, &real_rule, &pattern.replace, true, false);
            }
        }
    }

    // If the remark is empty after processing, restore the original
    if node.remark.is_empty() {
        node.remark = original_remark;
    }
}

/// Adds emoji to node remark based on regex matching
async fn add_emoji(node: &Proxy, emoji_array: &RegexMatchConfigs, extra: &ExtraSettings) -> String {
    for pattern in emoji_array {
        if !pattern.script.is_empty() {
            match extra
                .eval_get_emoji_node_remark(node, pattern.script.clone())
                .await
            {
                Ok(emoji) => {
                    return format!("{} {}", emoji, node.remark);
                }
                Err(e) => {
                    log::error!("Error adding emoji: {}", e);
                }
            }
            continue;
        }

        // Skip patterns with empty replace
        if pattern.replace.is_empty() {
            continue;
        }

        // Use apply_compiled_rule to handle complex matching rules
        if let Some(compiled_rule) = &pattern.compiled_rule {
            if apply_compiled_rule(compiled_rule, node) {
                return format!("{} {}", pattern.replace, node.remark);
            }
        }
    }

    node.remark.clone()
}

/// Preprocesses nodes before conversion
/// Based on the C++ preprocessNodes function
pub async fn preprocess_nodes(
    nodes: &mut Vec<Proxy>,
    extra: &mut ExtraSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    // Process each node
    for node in nodes.iter_mut() {
        // Remove emoji if needed
        if extra.remove_emoji {
            node.remark = trim(&remove_emoji(&node.remark)).to_string();
        }

        // Apply rename patterns
        node_rename(node, extra).await;

        // Add emoji if needed
        if extra.add_emoji {
            if extra
                .emoji_array
                .iter()
                .any(|pattern| !pattern.script.is_empty())
            {
                extra.init_js_context();
            }
            node.remark = add_emoji(node, &extra.emoji_array, extra).await;
        }
    }

    // Sort nodes if needed
    if extra.sort_flag && extra.authorized {
        info!("Sorting {} nodes", nodes.len());
        extra.eval_sort_nodes(nodes).await?;
    }

    debug!("Node preprocessing completed for {} nodes", nodes.len());
    Ok(())
}

/// Appends proxy type to node remark
pub fn append_type_to_remark(nodes: &mut Vec<Proxy>) {
    for node in nodes.iter_mut() {
        match node.proxy_type {
            ProxyType::Unknown => {}
            _ => {
                let type_str = node.proxy_type.to_string();
                node.remark = format!("{} ({})", node.remark, type_str);
            }
        }
    }
}
