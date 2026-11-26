//! Generic metadata table (groups of key/value entries) used by the viewer.

use serde::{Deserialize, Serialize};

/// One metadata entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Tag / property name.
    pub key: String,
    /// Human-readable value.
    pub value: String,
}

/// A named group of entries (e.g. "EXIF · Camera").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Group {
    /// Group name.
    pub name: String,
    /// Entries in source order.
    pub entries: Vec<Entry>,
}

impl Group {
    /// New empty group.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), entries: Vec::new() }
    }

    /// Adds an entry, truncating very long values for display.
    pub fn push(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let mut value: String = value.into();
        if value.len() > 4000 {
            let mut cut = 4000;
            while !value.is_char_boundary(cut) {
                cut -= 1;
            }
            value.truncate(cut);
            value.push_str(" …(truncated)");
        }
        self.entries.push(Entry { key: key.into(), value });
    }

    /// Looks up the first value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|e| e.key == key).map(|e| e.value.as_str())
    }
}
