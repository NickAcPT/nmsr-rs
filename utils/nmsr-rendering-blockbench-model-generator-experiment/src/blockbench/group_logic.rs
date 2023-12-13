use std::collections::HashMap;

use serde_json::{json, Value};
use uuid::Uuid;

pub enum BlockbenchGroupEntry {
    Root {
        elements: HashMap<String, BlockbenchGroupEntry>,
    },
    Group {
        name: String,
        elements: HashMap<String, BlockbenchGroupEntry>,
    },
    Entry(Uuid),
}

impl BlockbenchGroupEntry {
    pub fn to_value(&self) -> Value {
        match self {
            BlockbenchGroupEntry::Entry(uuid) => {
                json!(uuid)
            }
            BlockbenchGroupEntry::Root { elements } => {
                json!(elements
                    .iter()
                    .map(|(_, e)| e.to_value())
                    .collect::<Vec<_>>())
            }
            BlockbenchGroupEntry::Group { name, elements } => json!({
                "name": name,
                "children": elements.iter().map(|(_, e)| e.to_value()).collect::<Vec<_>>(),
            }),
        }
    }

    pub fn new_root() -> Self {
        Self::Root {
            elements: HashMap::new(),
        }
    }

    pub fn new_group(name: impl Into<String>) -> Self {
        Self::Group {
            name: name.into(),
            elements: HashMap::new(),
        }
    }

    pub fn add_entry(&mut self, uuid: Uuid) {
        match self {
            Self::Root { elements } | Self::Group { elements, .. } => {
                elements.insert(uuid.into(), Self::Entry(uuid));
            }
            Self::Entry(_) => panic!("Cannot add entry to entry"),
        }
    }

    pub fn add_or_get_group(&mut self, name: impl Into<String>) -> &mut Self {
        match self {
            Self::Root { elements } | Self::Group { elements, .. } => {
                let name: String = name.into();

                elements
                    .entry(name.clone())
                    .or_insert_with(|| Self::new_group(name))
            }
            Self::Entry(_) => panic!("Cannot add group to entry"),
        }
    }
}
