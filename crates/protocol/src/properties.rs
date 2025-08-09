use core::fmt;
use core::mem;

use alloc::string::String;

use std::collections::BTreeMap;

use crate::Prop;

/// Collection of properties.
#[derive(Default)]
pub struct Properties {
    properties: BTreeMap<&'static Prop, String>,
    modified: bool,
}

impl Properties {
    /// Create a new empty collection of properties.
    pub fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
            modified: false,
        }
    }

    /// Test if the properties collection has been modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Take the modification state of the properties.
    pub fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
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
    pub fn insert(&mut self, key: &'static Prop, value: impl AsRef<str>) {
        self.properties.insert(key, String::from(value.as_ref()));
        self.modified = true;
    }

    /// Remove and return a property by its key.
    pub fn remove(&mut self, key: &'static Prop) -> Option<String> {
        let value = self.properties.remove(key);
        self.modified |= value.is_some();
        value
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
