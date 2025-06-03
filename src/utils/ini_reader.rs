//! INI file reader implementation
//!
//! This module provides functionality for reading and parsing INI files,
//! similar to the C++ INIReader class in the original subconverter.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::{file_exists, file_get_async};

/// Error types for the INI reader
#[derive(Debug)]
pub enum IniReaderError {
    Empty,
    Duplicate,
    OutOfBound,
    NotExist,
    NotParsed,
    IoError(io::Error),
    None,
}

impl std::fmt::Display for IniReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IniReaderError::Empty => write!(f, "Empty document"),
            IniReaderError::Duplicate => write!(f, "Duplicate section"),
            IniReaderError::OutOfBound => write!(f, "Item exists outside of any section"),
            IniReaderError::NotExist => write!(f, "Target does not exist"),
            IniReaderError::NotParsed => write!(f, "Parse error"),
            IniReaderError::IoError(e) => write!(f, "IO error: {}", e),
            IniReaderError::None => write!(f, "No error"),
        }
    }
}

impl From<io::Error> for IniReaderError {
    fn from(error: io::Error) -> Self {
        IniReaderError::IoError(error)
    }
}

/// Custom INI reader implementation similar to C++ INIReader
pub struct IniReader {
    /// The parsed INI content (sections -> [(key, value)])
    content: HashMap<String, Vec<(String, String)>>,
    /// Whether the INI has been successfully parsed
    parsed: bool,
    /// The current section being operated on
    current_section: String,
    /// List of sections to exclude when parsing
    exclude_sections: HashSet<String>,
    /// List of sections to include when parsing (if empty, all sections are
    /// included)
    include_sections: HashSet<String>,
    /// List of sections to save directly without processing
    direct_save_sections: HashSet<String>,
    /// Ordered list of sections as they appear in the original file
    section_order: Vec<String>,
    /// Last error that occurred
    last_error: IniReaderError,
    /// Save any line within a section even if it doesn't follow the key=value
    /// format
    pub store_any_line: bool,
    /// Allow section titles to appear multiple times
    pub allow_dup_section_titles: bool,
    /// Keep empty sections while parsing
    pub keep_empty_section: bool,
    /// For storing lines before any section is defined
    isolated_items_section: String,
    /// Store isolated lines (lines before any section)
    pub store_isolated_line: bool,
}

impl Default for IniReader {
    fn default() -> Self {
        Self::new()
    }
}

impl IniReader {
    /// Create a new INI reader
    pub fn new() -> Self {
        IniReader {
            content: HashMap::new(),
            parsed: false,
            current_section: String::new(),
            exclude_sections: HashSet::new(),
            include_sections: HashSet::new(),
            direct_save_sections: HashSet::new(),
            section_order: Vec::new(),
            last_error: IniReaderError::None,
            store_any_line: false,
            allow_dup_section_titles: false,
            keep_empty_section: true,
            isolated_items_section: String::new(),
            store_isolated_line: false,
        }
    }

    /// Add a section to be excluded during parsing
    pub fn exclude_section(&mut self, section: &str) {
        self.exclude_sections.insert(section.to_string());
    }

    /// Add a section to be included during parsing
    pub fn include_section(&mut self, section: &str) {
        self.include_sections.insert(section.to_string());
    }

    /// Add a section to be saved directly without processing
    pub fn add_direct_save_section(&mut self, section: &str) {
        self.direct_save_sections.insert(section.to_string());
    }

    /// Set the section to store isolated items
    pub fn set_isolated_items_section(&mut self, section: &str) {
        self.isolated_items_section = section.to_string();
    }

    /// Erase all contents of the current section (keeps the section, just
    /// empties it)
    pub fn erase_section(&mut self) {
        if self.current_section.is_empty() {
            return;
        }

        if let Some(section_vec) = self.content.get_mut(&self.current_section) {
            section_vec.clear();
        }
    }

    /// Erase all items in a specific section but keep the section itself
    pub fn erase_section_by_name(&mut self, section: &str) {
        if !self.section_exist(section) {
            return;
        }

        if let Some(section_vec) = self.content.get_mut(section) {
            section_vec.clear();
        }
    }

    /// Check if a section should be ignored based on include/exclude settings
    fn should_ignore_section(&self, section: &str) -> bool {
        let excluded = self.exclude_sections.contains(section);
        let included = if self.include_sections.is_empty() {
            true
        } else {
            self.include_sections.contains(section)
        };

        excluded || !included
    }

    /// Check if a section should be saved directly without processing
    fn should_direct_save(&self, section: &str) -> bool {
        self.direct_save_sections.contains(section)
    }

    /// Create a new INI reader and parse a file
    pub async fn from_file(path: &str) -> Result<Self, IniReaderError> {
        let mut reader = IniReader::new();
        reader.parse_file(path).await?;
        Ok(reader)
    }

    /// Get the last error as a string
    pub fn get_last_error(&self) -> String {
        self.last_error.to_string()
    }

    /// Trim whitespace from a string
    fn trim_whitespace(s: &str) -> String {
        s.trim().to_string()
    }

    /// Process escape characters in a string
    fn process_escape_char(s: &mut String) {
        // Replace escape sequences with actual characters
        *s = s
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\t", "\t");
    }

    /// Process escape characters in reverse (for writing)
    fn process_escape_char_reverse(s: &mut String) {
        // Replace actual characters with escape sequences
        *s = s
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
    }

    /// Erase all data from the INI
    pub fn erase_all(&mut self) {
        self.content.clear();
        self.section_order.clear();
        self.current_section.clear();
        self.parsed = false;
    }

    /// Check if parsed successfully
    pub fn is_parsed(&self) -> bool {
        self.parsed
    }

    /// Parse INI content into the internal data structure
    pub fn parse(&mut self, content: &str) -> Result<(), IniReaderError> {
        // First clear all data
        self.erase_all();

        if content.is_empty() {
            self.last_error = IniReaderError::Empty;
            return Err(IniReaderError::Empty);
        }

        // Remove UTF-8 BOM if present
        let content = if content.starts_with("\u{FEFF}") {
            &content[3..]
        } else {
            content
        };

        let mut in_excluded_section = false;
        let mut in_direct_save_section = false;
        let mut in_isolated_section = false;

        let mut cur_section = String::new();
        let mut item_group = Vec::new();
        let mut read_sections = Vec::new();

        // Check if we need to handle isolated items
        if self.store_isolated_line && !self.isolated_items_section.is_empty() {
            cur_section = self.isolated_items_section.clone();
            in_excluded_section = self.should_ignore_section(&cur_section);
            in_direct_save_section = self.should_direct_save(&cur_section);
            in_isolated_section = true;
        }

        // Process each line
        for line in content.lines() {
            let line = Self::trim_whitespace(line);

            // Skip empty lines and comments
            if line.is_empty()
                || line.starts_with(';')
                || line.starts_with('#')
                || line.starts_with("//")
            {
                continue;
            }

            let mut line = line.to_string();

            // Process escape characters
            Self::process_escape_char(&mut line);

            // Check if it's a section header [section]
            if line.starts_with('[') && line.ends_with(']') && line.len() >= 3 {
                let this_section = line[1..line.len() - 1].to_string();
                in_excluded_section = self.should_ignore_section(&this_section);
                in_direct_save_section = self.should_direct_save(&this_section);

                // Save previous section if not empty
                if !cur_section.is_empty() && (self.keep_empty_section || !item_group.is_empty()) {
                    if self.content.contains_key(&cur_section) {
                        // Handle duplicate section
                        if self.allow_dup_section_titles
                            || self.content.get(&cur_section).unwrap().is_empty()
                        {
                            // Merge with existing section
                            if let Some(existing_items) = self.content.get_mut(&cur_section) {
                                existing_items.extend(item_group.drain(..));
                            }
                        } else {
                            self.last_error = IniReaderError::Duplicate;
                            return Err(IniReaderError::Duplicate);
                        }
                    } else if !in_isolated_section || self.isolated_items_section != this_section {
                        if !item_group.is_empty() {
                            read_sections.push(cur_section.clone());
                        }

                        if !self.section_order.contains(&cur_section) {
                            self.section_order.push(cur_section.clone());
                        }

                        self.content.insert(cur_section.clone(), item_group);
                    }
                }

                in_isolated_section = false;
                cur_section = this_section;
                item_group = Vec::new();
            }
            // Handle normal lines within a section
            else if !in_excluded_section && !cur_section.is_empty() {
                let pos_equal = line.find('=');

                // Handle lines without equals sign (or direct save sections)
                if (self.store_any_line && pos_equal.is_none()) || in_direct_save_section {
                    item_group.push(("{NONAME}".to_string(), line));
                }
                // Handle key=value pairs
                else if let Some(pos) = pos_equal {
                    let item_name = line[0..pos].trim().to_string();
                    let item_value = if pos + 1 < line.len() {
                        line[pos + 1..].trim().to_string()
                    } else {
                        String::new()
                    };

                    item_group.push((item_name, item_value));
                }
            } else if cur_section.is_empty() {
                // Items outside of any section
                self.last_error = IniReaderError::OutOfBound;
                return Err(IniReaderError::OutOfBound);
            }

            // Check if all included sections have been read
            if !self.include_sections.is_empty()
                && read_sections
                    .iter()
                    .all(|s| self.include_sections.contains(s))
            {
                break;
            }
        }

        // Save the final section
        if !cur_section.is_empty() && (self.keep_empty_section || !item_group.is_empty()) {
            if self.content.contains_key(&cur_section) {
                if self.allow_dup_section_titles || in_isolated_section {
                    // Merge with existing section
                    if let Some(existing_items) = self.content.get_mut(&cur_section) {
                        existing_items.extend(item_group.drain(..));
                    }
                } else if !self.content.get(&cur_section).unwrap().is_empty() {
                    self.last_error = IniReaderError::Duplicate;
                    return Err(IniReaderError::Duplicate);
                }
            } else if !in_isolated_section || self.isolated_items_section != cur_section {
                if !item_group.is_empty() {
                    read_sections.push(cur_section.clone());
                }

                if !self.section_order.contains(&cur_section) {
                    self.section_order.push(cur_section.clone());
                }

                self.content.insert(cur_section, item_group);
            }
        }

        self.parsed = true;
        self.last_error = IniReaderError::None;
        Ok(())
    }

    /// Parse an INI file
    pub async fn parse_file(&mut self, path: &str) -> Result<(), IniReaderError> {
        // Check if file exists
        if !file_exists(path).await {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        // Read the file
        let content = file_get_async(path, None).await?;

        // Parse the content
        self.parse(&content)
    }

    /// Check if a section exists
    pub fn section_exist(&self, section: &str) -> bool {
        self.content.contains_key(section)
    }

    /// Get the count of sections
    pub fn section_count(&self) -> usize {
        self.content.len()
    }

    /// Get all section names
    pub fn get_section_names(&self) -> &[String] {
        &self.section_order
    }

    /// Set the current section
    pub fn set_current_section(&mut self, section: &str) {
        self.current_section = section.to_string();
    }

    /// Enter a section with the given name
    pub fn enter_section(&mut self, section: &str) -> Result<(), IniReaderError> {
        if !self.section_exist(section) {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        self.current_section = section.to_string();
        self.last_error = IniReaderError::None;
        Ok(())
    }

    /// Check if an item exists in the given section
    pub fn item_exist(&self, section: &str, item_name: &str) -> bool {
        if !self.section_exist(section) {
            return false;
        }

        self.content
            .get(section)
            .map(|items| items.iter().any(|(key, _)| key == item_name))
            .unwrap_or(false)
    }

    /// Check if an item exists in the current section
    pub fn item_exist_current(&self, item_name: &str) -> bool {
        if self.current_section.is_empty() {
            return false;
        }

        self.item_exist(&self.current_section, item_name)
    }

    /// Check if an item with given prefix exists in the section
    pub fn item_prefix_exists(&self, section: &str, prefix: &str) -> bool {
        if !self.section_exist(section) {
            return false;
        }

        if let Some(items) = self.content.get(section) {
            return items.iter().any(|(key, _)| key.starts_with(prefix));
        }

        false
    }

    /// Check if an item with given prefix exists in the current section
    pub fn item_prefix_exist(&self, prefix: &str) -> bool {
        if self.current_section.is_empty() {
            return false;
        }

        self.item_prefix_exists(&self.current_section, prefix)
    }

    /// Get all items in a section
    pub fn get_items(&self, section: &str) -> Result<Vec<(String, String)>, IniReaderError> {
        if !self.parsed {
            return Err(IniReaderError::NotParsed);
        }

        if !self.section_exist(section) {
            return Err(IniReaderError::NotExist);
        }

        Ok(self.content.get(section).cloned().unwrap_or_default())
    }

    /// Get all items with the same name prefix in a section
    pub fn get_all(&self, section: &str, item_name: &str) -> Result<Vec<String>, IniReaderError> {
        if !self.parsed {
            return Err(IniReaderError::NotParsed);
        }

        if !self.section_exist(section) {
            return Err(IniReaderError::NotExist);
        }

        let mut results = Vec::new();

        if let Some(items) = self.content.get(section) {
            for (key, value) in items {
                if key.starts_with(item_name) {
                    results.push(value.clone());
                }
            }
        }

        Ok(results)
    }

    /// Get all items with the same name prefix in the current section
    pub fn get_all_current(&self, item_name: &str) -> Result<Vec<String>, IniReaderError> {
        if self.current_section.is_empty() {
            return Err(IniReaderError::NotExist);
        }

        self.get_all(&self.current_section, item_name)
    }

    /// Get an item with the exact same name in the given section
    pub fn get(&self, section: &str, item_name: &str) -> String {
        if !self.parsed || !self.section_exist(section) {
            return String::new();
        }

        self.content
            .get(section)
            .and_then(|items| items.iter().find(|(key, _)| key == item_name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    }

    /// Get an item with the exact same name in the current section
    pub fn get_current(&self, item_name: &str) -> String {
        if self.current_section.is_empty() {
            return String::new();
        }

        self.get(&self.current_section, item_name)
    }

    /// Get a boolean value from the given section
    pub fn get_bool(&self, section: &str, item_name: &str) -> bool {
        self.get(section, item_name) == "true"
    }

    /// Get a boolean value from the current section
    pub fn get_bool_current(&self, item_name: &str) -> bool {
        self.get_current(item_name) == "true"
    }

    /// Get an integer value from the given section
    pub fn get_int(&self, section: &str, item_name: &str) -> i32 {
        self.get(section, item_name).parse::<i32>().unwrap_or(0)
    }

    /// Get an integer value from the current section
    pub fn get_int_current(&self, item_name: &str) -> i32 {
        self.get_current(item_name).parse::<i32>().unwrap_or(0)
    }

    /// Set a value in the given section
    pub fn set(
        &mut self,
        section: &str,
        item_name: &str,
        item_val: &str,
    ) -> Result<(), IniReaderError> {
        if section.is_empty() {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        if !self.parsed {
            self.parsed = true;
        }

        // If section is {NONAME}, we're setting key directly to the current section
        let real_section = if section == "{NONAME}" {
            if self.current_section.is_empty() {
                self.last_error = IniReaderError::NotExist;
                return Err(IniReaderError::NotExist);
            }
            &self.current_section
        } else {
            section
        };

        // Add section if it doesn't exist
        if !self.section_exist(real_section) {
            self.section_order.push(real_section.to_string());
            self.content.insert(real_section.to_string(), Vec::new());
        }

        // Update our content HashMap
        if let Some(section_vec) = self.content.get_mut(real_section) {
            section_vec.push((item_name.to_string(), item_val.to_string()));
        }

        self.last_error = IniReaderError::None;
        Ok(())
    }

    /// Set a value in the current section
    pub fn set_current(&mut self, item_name: &str, item_val: &str) -> Result<(), IniReaderError> {
        if self.current_section.is_empty() {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        // Handle the special case where item_name is {NONAME}
        if item_name == "{NONAME}" {
            return self.set_current_with_noname(item_val);
        }

        self.set(&self.current_section.clone(), item_name, item_val)
    }

    /// Set a boolean value in the given section
    pub fn set_bool(
        &mut self,
        section: &str,
        item_name: &str,
        item_val: bool,
    ) -> Result<(), IniReaderError> {
        self.set(section, item_name, if item_val { "true" } else { "false" })
    }

    /// Set a boolean value in the current section
    pub fn set_bool_current(
        &mut self,
        item_name: &str,
        item_val: bool,
    ) -> Result<(), IniReaderError> {
        if self.current_section.is_empty() {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        let value = if item_val { "true" } else { "false" };
        self.set(&self.current_section.clone(), item_name, value)
    }

    /// Set an integer value in the given section
    pub fn set_int(
        &mut self,
        section: &str,
        item_name: &str,
        item_val: i32,
    ) -> Result<(), IniReaderError> {
        self.set(section, item_name, &item_val.to_string())
    }

    /// Set an integer value in the current section
    pub fn set_int_current(
        &mut self,
        item_name: &str,
        item_val: i32,
    ) -> Result<(), IniReaderError> {
        if self.current_section.is_empty() {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        self.set(
            &self.current_section.clone(),
            item_name,
            &item_val.to_string(),
        )
    }

    /// Export the INI to a string
    pub fn to_string(&self) -> String {
        if !self.parsed {
            return String::new();
        }

        let mut result = String::new();

        for section_name in &self.section_order {
            // Add section header
            result.push_str(&format!("[{}]\n", section_name));

            if let Some(section) = self.content.get(section_name) {
                if section.is_empty() {
                    result.push('\n');
                    continue;
                }

                // Add all items in this section
                for (key, value) in section {
                    let mut value = value.clone();
                    Self::process_escape_char_reverse(&mut value);

                    if key != "{NONAME}" {
                        result.push_str(&format!("{}={}\n", key, value));
                    } else {
                        result.push_str(&format!("{}\n", value));
                    }
                }

                // Add extra newline after section
                result.push('\n');
            }
        }

        result
    }

    /// Export the INI to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        if !self.parsed {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "INI not parsed"));
        }

        let content = self.to_string();
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Set a value in the current section with {NONAME} key
    /// This is used in patterns like set_current("{NONAME}", "value")
    pub fn set_current_with_noname(&mut self, item_val: &str) -> Result<(), IniReaderError> {
        if self.current_section.is_empty() {
            self.last_error = IniReaderError::NotExist;
            return Err(IniReaderError::NotExist);
        }

        self.set(&self.current_section.clone(), "{NONAME}", item_val)
    }
}
