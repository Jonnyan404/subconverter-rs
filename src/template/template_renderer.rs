use crate::api::SubconverterQuery;
use crate::utils::{file_exists, file_get_async};
use crate::Settings;
use log::{debug, error};
use minijinja::{
    context, escape_formatter, Environment, Error as JinjaError, ErrorKind, UndefinedBehavior,
    Value,
};
use serde::Serialize;
use std::collections::HashMap;

/// Template arguments container
#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplateArgs {
    /// Global variables
    pub global_vars: HashMap<String, String>,

    /// Request parameters
    pub request_params: SubconverterQuery,

    /// Local variables
    pub local_vars: HashMap<String, String>,

    /// Node list variables
    pub node_list: HashMap<String, String>,
}

/// Render a template with the given arguments
///
/// # Arguments
/// * `content` - The template content
/// * `args` - Template arguments
/// * `include_scope` - The directory scope for included templates
///
/// # Returns
/// * `Ok(String)` - The rendered template
/// * `Err(String)` - Error message if rendering fails
pub fn render_template(
    content: &str,
    args: &TemplateArgs,
    _include_scope: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // let env_lock = match TEMPLATE_ENV.lock() {
    //     Ok(env) => env,
    //     Err(e) => {
    //         return Err(format!("Failed to acquire template environment lock: {}", e).into());
    //     }
    // };

    // Create a new environment for this template
    let mut env = Environment::new();

    // Copy settings from global environment
    env.set_formatter(escape_formatter);
    env.set_undefined_behavior(UndefinedBehavior::Chainable);

    // Add the same filters and functions
    env.add_filter("trim", filter_trim);
    env.add_filter("trim_of", filter_trim_of);
    env.add_filter("url_encode", filter_url_encode);
    env.add_filter("url_decode", filter_url_decode);
    env.add_filter("replace", filter_replace);
    env.add_filter("find", filter_find);

    env.add_function("getLink", fn_get_link);
    env.add_function("startsWith", fn_starts_with);
    env.add_function("endsWith", fn_ends_with);
    env.add_function("bool", fn_to_bool);
    env.add_function("string", fn_to_string);

    env.add_function("default", fn_default);
    // env.add_function("fetch", fn_web_get);

    // Build context object
    let mut global_vars = HashMap::new();
    for (key, value) in &args.global_vars {
        global_vars.insert(key.clone(), value.clone());
    }

    // Create full context with all variables
    let context = context!(
        global => global_vars,
        request => args.request_params,
        local => args.local_vars,
        node_list => args.node_list
    );

    debug!("Template context: {:?}", context);

    // Parse and render the template
    match env.template_from_str(content) {
        Ok(template) => match template.render(context) {
            Ok(result) => Ok(result),
            Err(e) => {
                let error_msg = format!("Template render failed! Reason: {}", e);
                error!("{}", error_msg);
                Err(Box::new(e))
            }
        },
        Err(e) => {
            let error_msg = format!("Failed to parse template: {}", e);
            error!("{}", error_msg);
            Err(Box::new(e))
        }
    }
}

/// Render a template from a file with the given arguments
///
/// # Arguments
/// * `path` - Path to the template file
/// * `args` - Template arguments
/// * `include_scope` - The directory scope for included templates
///
/// # Returns
/// * `Ok(String)` - The rendered template
/// * `Err(String)` - Error message if rendering fails
pub async fn render_template_file(
    path: &str,
    args: &TemplateArgs,
    include_scope: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let content;
    if file_exists(path).await {
        content = file_get_async(
            path,
            if include_scope.is_empty() {
                None
            } else {
                Some(include_scope)
            },
        )
        .await?;
    } else {
        return Err(format!("Template file not found: {}", path).into());
    }

    render_template(&content, args, include_scope)
}

// Filter implementations

fn filter_trim(value: Value) -> Result<String, JinjaError> {
    let s = value.to_string();
    Ok(s.trim().to_string())
}

fn filter_trim_of(value: Value, chars: Value) -> Result<String, JinjaError> {
    let s = value.to_string();
    let chars_str = chars.to_string();

    if chars_str.is_empty() {
        return Ok(s);
    }

    let first_char = chars_str.chars().next().unwrap();
    Ok(s.trim_matches(first_char).to_string())
}

fn filter_url_encode(value: Value) -> Result<String, JinjaError> {
    let s = value.to_string();
    Ok(urlencoding::encode(&s).to_string())
}

fn filter_url_decode(value: Value) -> Result<String, JinjaError> {
    let s = value.to_string();
    match urlencoding::decode(&s) {
        Ok(decoded) => Ok(decoded.to_string()),
        Err(e) => Err(JinjaError::new(
            ErrorKind::InvalidOperation,
            format!("URL decode error: {}", e),
        )),
    }
}

fn filter_replace(value: Value, pattern: Value, replacement: Value) -> Result<String, JinjaError> {
    let s = value.to_string();
    let pattern_str = pattern.to_string();
    let replacement_str = replacement.to_string();

    if pattern_str.is_empty() || s.is_empty() {
        return Ok(s);
    }

    // Use regex for replacement
    match regex::Regex::new(&pattern_str) {
        Ok(re) => Ok(re.replace_all(&s, replacement_str.as_str()).to_string()),
        Err(e) => Err(JinjaError::new(
            ErrorKind::InvalidOperation,
            format!("Invalid regex pattern: {}", e),
        )),
    }
}

fn filter_find(value: Value, pattern: Value) -> Result<bool, JinjaError> {
    let s = value.to_string();
    let pattern_str = pattern.to_string();

    if pattern_str.is_empty() || s.is_empty() {
        return Ok(false);
    }

    // Use regex for finding
    match regex::Regex::new(&pattern_str) {
        Ok(re) => Ok(re.is_match(&s)),
        Err(e) => Err(JinjaError::new(
            ErrorKind::InvalidOperation,
            format!("Invalid regex pattern: {}", e),
        )),
    }
}

// Function implementations

fn fn_get_link(path: Value) -> Result<String, JinjaError> {
    let path_str = path.to_string();
    let settings = Settings::current();
    Ok(format!("{}{}", settings.managed_config_prefix, path_str))
}

fn fn_starts_with(s: Value, prefix: Value) -> Result<bool, JinjaError> {
    let s_str = s.to_string();
    let prefix_str = prefix.to_string();
    Ok(s_str.starts_with(&prefix_str))
}

fn fn_ends_with(s: Value, suffix: Value) -> Result<bool, JinjaError> {
    let s_str = s.to_string();
    let suffix_str = suffix.to_string();
    Ok(s_str.ends_with(&suffix_str))
}

fn fn_to_bool(s: Value) -> Result<bool, JinjaError> {
    let s_str = s.to_string().to_lowercase();
    Ok(s_str == "true" || s_str == "1")
}

fn fn_to_string(n: Value) -> Result<String, JinjaError> {
    Ok(n.to_string())
}

fn fn_default(value: Value, default: Value) -> Result<String, JinjaError> {
    if value.is_undefined() || value.is_none() {
        Ok(default.to_string())
    } else {
        Ok(value.to_string())
    }
}
