use core::fmt;
use core::mem;

use alloc::vec::Vec;

use std::collections::btree_map::{self, BTreeMap};

use anyhow::Result;
use pod::{AsSlice, DynamicBuf};
use protocol::{flags, id};

use crate::PortParam;

#[derive(Debug)]
struct Entry {
    values: Vec<PortParam<DynamicBuf>>,
    flags: flags::ParamFlags,
}

impl Default for Entry {
    #[inline]
    fn default() -> Self {
        Self {
            values: Vec::with_capacity(1),
            flags: flags::ParamFlags::NONE,
        }
    }
}

/// A collection of parameters for pipewire objects.
pub struct Parameters {
    values: BTreeMap<id::Param, Entry>,
    modified: bool,
}

impl Parameters {
    /// Construct a new collection of parameters.
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            modified: false,
        }
    }

    /// Test if the parameters collection has been modified.
    pub(crate) fn is_modified(&self) -> bool {
        self.modified
    }

    /// Take the modified state of the parameters.
    pub(crate) fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }

    /// Set a parameter flag.
    pub fn set_readable(&mut self, id: id::Param) {
        self.values.entry(id).or_default().flags |= flags::ParamFlags::READ;
        self.modified = true;
    }

    /// Set that a parameter is writable.
    pub fn set_writable(&mut self, id: id::Param) {
        self.values.entry(id).or_default().flags |= flags::ParamFlags::WRITE;
        self.modified = true;
    }

    /// Set a parameter.
    ///
    /// This overrides all values for the parameter and marks the collection as
    /// modified.
    #[inline]
    pub fn set<V, S>(&mut self, id: id::Param, values: V) -> Result<()>
    where
        V: IntoIterator<IntoIter: ExactSizeIterator>,
        PortParam<S>: From<V::Item>,
        S: AsSlice,
    {
        let e = self.values.entry(id).or_default();

        for param in values {
            let param = PortParam::from(param);

            e.values.push(PortParam::with_flags(
                param.value.as_ref().to_owned()?,
                param.flags,
            ));
        }

        e.flags |= flags::ParamFlags::READ;
        self.modified = true;
        Ok(())
    }

    /// Push a parameter.
    ///
    /// This will append to the given parameter and mark the collection as
    /// modified.
    #[inline]
    pub fn push<S, V>(&mut self, value: V) -> Result<()>
    where
        S: AsSlice,
        PortParam<S>: From<V>,
    {
        let value = PortParam::from(value);
        let id = value.value.object_id();
        let e = self.values.entry(id).or_default();

        e.values.push(PortParam::with_flags(
            value.value.as_ref().to_owned()?,
            value.flags,
        ));

        e.flags |= flags::ParamFlags::READ;
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port and return the values of the removed
    /// parameter if it exists.
    #[inline]
    pub fn remove(&mut self, id: id::Param) -> bool {
        let e = self.values.entry(id).or_default();
        let removed = !e.values.is_empty();

        e.values.clear();
        // If we remove a parameter it is no longer readable.
        e.flags ^= flags::ParamFlags::READ;

        self.modified = true;
        removed
    }

    /// Get the values of a parameter.
    pub fn get(&self, id: id::Param) -> &[PortParam<DynamicBuf>] {
        match self.values.get(&id) {
            Some(entry) => entry.values.as_slice(),
            None => &[],
        }
    }

    /// Get parameters from the port.
    pub(crate) fn values(&self) -> impl ExactSizeIterator<Item = &[PortParam<DynamicBuf>]> {
        self.values.values().map(|e| e.values.as_slice())
    }

    /// Get parameters from the port.
    pub(crate) fn flags(&self) -> impl ExactSizeIterator<Item = (id::Param, flags::ParamFlags)> {
        self.values.iter().map(|(id, e)| (*id, e.flags))
    }
}

impl fmt::Debug for Parameters {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parameters")
            .field("values", &self.values)
            .field("modified", &self.modified)
            .finish()
    }
}

impl Default for Parameters {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
