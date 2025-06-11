/// 清理 IPv6 地址，移除方括号
fn clean_ipv6_address(rule_content: &str) -> String {
    // 匹配 [IPv6地址]/prefix 格式
    if rule_content.contains('[') && rule_content.contains(']') && rule_content.contains('/') {
        if let Some(start) = rule_content.find('[') {
            if let Some(end) = rule_content.find(']') {
                if start < end {
                    let before = &rule_content[..start];
                    let ipv6 = &rule_content[start + 1..end];
                    let after = &rule_content[end + 1..];
                    return format!("{}{}{}", before, ipv6, after);
                }
            }
        }
    }
    rule_content.to_string()
}

/// Transforms a rule to a common format for use in different proxy clients
///
/// # Arguments
///
/// * `input` - The rule to transform
/// * `group` - The proxy group to assign
/// * `no_resolve_only` - Whether to only keep no-resolve parameter
///
/// # Returns
///
/// The transformed rule as a string
pub fn transform_rule_to_common(input: &str, group: &str, no_resolve_only: bool) -> String {
    let mut parts = ["", "", "", ""]; // Pre-allocate array with 4 elements like C++ version
    let mut part_count = 0;

    // Split the input by comma and fill the parts array
    for (i, part) in input.split(',').enumerate() {
        if i < 4 {
            parts[i] = part;
            part_count = i + 1;
        } else {
            break;
        }
    }

    if part_count < 2 {
        // Single part rule, just add group
        let cleaned_rule = clean_ipv6_address(parts[0]);
        format!("{},{}", cleaned_rule, group)
    } else {
        // Multi-part rule
        let cleaned_rule_type = parts[0];
        let cleaned_rule_content = clean_ipv6_address(parts[1]);
        let mut result = format!("{},{},{}", cleaned_rule_type, cleaned_rule_content, group);

        // Add options like no-resolve if present and applicable
        if part_count > 2 && (!no_resolve_only || parts[2] == "no-resolve") {
            result = format!("{},{}", result, parts[2]);
        }

        result
    }
}
