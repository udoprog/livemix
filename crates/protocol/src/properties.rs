use core::borrow::Borrow;
use core::fmt;
use core::iter::Map;
use core::mem;
use std::collections::btree_map;

use alloc::string::String;

use std::collections::BTreeMap;

use crate::Prop;

/// Collection of properties.
#[derive(Default)]
pub struct Properties {
    properties: BTreeMap<String, String>,
    modified: bool,
}

impl Properties {
    /// Create a new empty collection of properties.
    pub const fn new() -> Self {
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
        self.properties
            .iter()
            .map(|(k, v)| (Prop::new(k.as_str()), v.as_str()))
    }

    /// Insert a property into the collection.
    pub fn insert(&mut self, key: impl AsRef<Prop>, value: impl AsRef<str>) -> bool {
        let key = key.as_ref().as_str();
        let value = value.as_ref();

        let old = self
            .properties
            .insert(String::from(key), String::from(value));

        let Some(old) = old else {
            self.modified = true;
            return true;
        };

        if old == value {
            return false;
        }

        self.modified = true;
        true
    }

    /// Remove and return a property by its key.
    pub fn remove<K>(&mut self, key: &K) -> Option<String>
    where
        K: ?Sized + Ord,
        String: Borrow<K>,
    {
        let value = self.properties.remove(key);
        self.modified |= value.is_some();
        value
    }

    /// Get the value of a property by its key.
    pub fn get<K>(&self, key: &K) -> Option<&str>
    where
        K: ?Sized + Ord,
        String: Borrow<K>,
    {
        self.properties.get(key).map(|s| s.as_str())
    }

    /// Extend this collection of properties with another.
    ///
    /// Returns `true` if any properties were added or modified.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::Properties;
    ///
    /// let mut props = Properties::new();
    /// props.insert("key1", "value1");
    ///
    /// let mut other = Properties::new();
    /// other.insert("key2", "value2");
    ///
    /// assert!(props.extend(&other));
    /// assert_eq!(props.len(), 2);
    /// assert_eq!(props.get("key1"), Some("value1"));
    /// assert_eq!(props.get("key2"), Some("value2"));
    ///
    /// assert!(!props.extend(&other));
    ///
    /// let mut another = Properties::new();
    /// another.insert("key1", "new_value1");
    ///
    /// assert!(props.extend(&another));
    /// assert_eq!(props.len(), 2);
    /// assert_eq!(props.get("key1"), Some("new_value1"));
    /// assert_eq!(props.get("key2"), Some("value2"));
    /// ```
    pub fn extend<K, V>(&mut self, iter: impl IntoIterator<Item = (K, V)>) -> bool
    where
        K: AsRef<Prop>,
        V: AsRef<str>,
    {
        let mut modified = false;

        for (key, value) in iter {
            modified |= self.insert(key, value);
        }

        modified
    }
}

impl fmt::Debug for Properties {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.properties.fmt(f)
    }
}

/// The iterator produced by iterating over a borrowed [`Properties`].
pub type Iter<'a> =
    Map<btree_map::Iter<'a, String, String>, fn((&'a String, &'a String)) -> (&'a Prop, &'a str)>;

impl<'a> IntoIterator for &'a Properties {
    type Item = (&'a Prop, &'a str);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.properties
            .iter()
            .map(|(k, v)| (Prop::new(k.as_str()), v.as_str()))
    }
}
