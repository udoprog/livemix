use core::fmt;

use alloc::string::String;

use std::collections::BTreeMap;

use crate::Prop;

/// Collection of properties.
#[derive(Default)]
pub struct Properties {
    properties: BTreeMap<&'static Prop, String>,
}

impl Properties {
    /// Create a new empty collection of properties.
    pub fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
        }
    }

    /// Get the number of properties in the collection.
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Iterate over the properties in the collection.
    pub fn iter(&self) -> impl Iterator<Item = (&Prop, &str)> {
        self.properties.iter().map(|(k, v)| (*k, v.as_str()))
    }

    /// Insert a property into the collection.
    pub fn insert(&mut self, key: &'static Prop, value: String) {
        self.properties.insert(key, value);
    }

    /// Remove and return a property by its key.
    pub fn remove(&mut self, key: &'static Prop) -> Option<String> {
        self.properties.remove(key)
    }

    /// Get the value of a property by its key.
    pub fn get(&self, key: &Prop) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }
}

impl fmt::Debug for Properties {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.properties.fmt(f)
    }
}
