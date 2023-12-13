use serde_json::{json, Value};
use uuid::Uuid;

pub enum BlockbenchGroupEntry {
    Root {
        elements: Vec<BlockbenchGroupEntry>,
    },
    Group {
        name: String,
        elements: Vec<BlockbenchGroupEntry>,
    },
    Entry(Uuid),
}

impl BlockbenchGroupEntry {
    pub fn to_value(&self) -> Value {
        match self {
            BlockbenchGroupEntry::Entry(uuid) => {
                json!({
                    "uuid": uuid,
                })
            }
            BlockbenchGroupEntry::Root { elements } => {
                json!(elements.iter().map(|e| e.to_value()).collect::<Vec<_>>())
            }
            BlockbenchGroupEntry::Group { name, elements } => json!({
                "name": name,
                "children": elements.iter().map(|e| e.to_value()).collect::<Vec<_>>(),
            }),
        }
    }

    pub fn new_root() -> Self {
        Self::Root { elements: vec![] }
    }
    
    pub fn add_element(&mut self, element: Self) {
        match self {
            Self::Root { elements } | Self::Group { elements, .. } => {
                elements.push(element);
            }
            Self::Entry(_) => panic!("Cannot add element to entry"),
        }
    }

    pub fn add_new_element(&mut self, uuid: Uuid) {
        self.add_element(Self::Entry(uuid));
    }

    pub fn get_group(&mut self, name: &str) -> Option<&mut Self> {
        match self {
            Self::Root { elements } | Self::Group { elements, .. } => {
                elements.iter_mut().find_map(|e| match e {
                    Self::Group { name: n, .. } if n == name => Some(e),
                    _ => None,
                })
            }
            Self::Entry(_) => None,
        }
    }
}
