use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    Daily,
    FreeForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MetadataValue {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub entry_type: EntryType,
    pub template: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, MetadataValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Entry {
    pub fn new_daily() -> Self {
        let now = Utc::now();
        Entry {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            entry_type: EntryType::Daily,
            template: "blank".to_string(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            title: None,
        }
    }

    pub fn new_free_form(title: Option<String>) -> Self {
        let now = Utc::now();
        Entry {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            entry_type: EntryType::FreeForm,
            template: "blank".to_string(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            title,
        }
    }

    /// Serialize to `---\n<yaml>\n---\n\n<body>` format.
    pub fn to_file_content(&self, body: &str) -> Result<String, StorageError> {
        let yaml = serde_yaml::to_string(self)
            .map_err(|e| StorageError::Parse(e.to_string()))?;
        Ok(format!("---\n{}---\n\n{}", yaml, body))
    }

    /// Parse from `---\n<yaml>\n---\n\n<body>` format.
    pub fn from_file_content(content: &str) -> Result<(Entry, String), StorageError> {
        let content = content.trim_start_matches('\u{FEFF}'); // strip BOM if present
        if !content.starts_with("---\n") {
            return Err(StorageError::Parse("Missing YAML frontmatter opening ---".into()));
        }
        let rest = &content[4..];
        let end = rest
            .find("\n---\n")
            .ok_or_else(|| StorageError::Parse("Missing YAML frontmatter closing ---".into()))?;
        let frontmatter = &rest[..end];
        let body = rest[end + 5..].trim_start_matches('\n').to_string();

        let entry: Entry = serde_yaml::from_str(frontmatter)
            .map_err(|e| StorageError::Parse(e.to_string()))?;
        Ok((entry, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_daily() {
        let mut entry = Entry::new_daily();
        entry.tags = vec!["work".into(), "morning".into()];
        entry.metadata.insert("mood".into(), MetadataValue::Number(7.0));
        entry.metadata.insert("weather".into(), MetadataValue::Text("sunny".into()));
        let body = "Today was productive.";

        let file_content = entry.to_file_content(body).unwrap();
        let (parsed, parsed_body) = Entry::from_file_content(&file_content).unwrap();

        assert_eq!(entry, parsed);
        assert_eq!(body, parsed_body);
    }

    #[test]
    fn round_trip_free_form() {
        let mut entry = Entry::new_free_form(Some("Dream Log".into()));
        entry.template = "dream_log".into();
        let body = "I was flying over a city.";

        let file_content = entry.to_file_content(body).unwrap();
        let (parsed, parsed_body) = Entry::from_file_content(&file_content).unwrap();

        assert_eq!(entry, parsed);
        assert_eq!(body, parsed_body);
    }

    #[test]
    fn missing_frontmatter_fails() {
        let result = Entry::from_file_content("just some text without frontmatter");
        assert!(result.is_err());
    }
}
