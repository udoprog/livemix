use core::fmt;
use core::mem;

use alloc::vec::Vec;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use anyhow::Result;
use pod::{AsSlice, DynamicBuf};
use protocol::{flags, id};

use crate::PortParam;

/// A collection of parameters for pipewire objects.
pub struct Parameters {
    pub(crate) values: BTreeMap<id::Param, Vec<PortParam<DynamicBuf>>>,
    pub(crate) flags: BTreeMap<id::Param, flags::ParamFlag>,
    pub(crate) modified: bool,
}

impl Parameters {
    /// Construct a new collection of parameters.
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            flags: BTreeMap::new(),
            modified: false,
        }
    }

    /// Test if the parameters collection has been modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Take the modified state of the parameters.
    pub fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }

    /// Set a parameter flag.
    pub fn set_read(&mut self, id: id::Param) {
        self.set_flag(id, flags::ParamFlag::READ);
    }

    /// Set that a parameter is writable.
    pub fn set_write(&mut self, id: id::Param) {
        self.set_flag(id, flags::ParamFlag::WRITE);
    }

    /// Set a parameter on the port to the given values.
    #[inline]
    pub fn set_param<V, S>(&mut self, id: id::Param, values: V) -> Result<()>
    where
        V: IntoIterator<IntoIter: ExactSizeIterator>,
        PortParam<S>: From<V::Item>,
        S: AsSlice,
    {
        let mut iter = values.into_iter();
        let mut params = Vec::with_capacity(iter.len());

        for param in iter {
            let param = PortParam::from(param);

            params.push(PortParam::with_flags(
                param.value.as_ref().to_owned()?,
                param.flags,
            ));
        }

        self.values.insert(id, params);
        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Push a parameter.
    ///
    /// This will append the value to the existing set of parameters of the
    /// given type.
    #[inline]
    pub fn push_param<S, V>(&mut self, value: V) -> Result<()>
    where
        S: AsSlice,
        PortParam<S>: From<V>,
    {
        let value = PortParam::from(value);
        let id = value.value.object_id();

        self.values
            .entry(id)
            .or_default()
            .push(PortParam::with_flags(
                value.value.as_ref().to_owned()?,
                value.flags,
            ));

        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port and return the values of the removed
    /// parameter if it exists.
    #[inline]
    pub fn remove_param(&mut self, id: id::Param) -> Option<Vec<PortParam>> {
        let param = self.values.remove(&id)?;

        // If we remove a parameter it is no longer readable.
        let flag = self.flags.entry(id).or_default();
        *flag ^= flags::ParamFlag::READ;

        self.modified = true;
        Some(param)
    }

    /// Get the values of a parameter.
    pub fn get_param(&self, id: id::Param) -> &[PortParam<DynamicBuf>] {
        self.values.get(&id).map(Vec::as_slice).unwrap_or_default()
    }

    /// Get parameters from the port.
    pub(crate) fn param_values(&self) -> &BTreeMap<id::Param, Vec<PortParam<impl AsSlice>>> {
        &self.values
    }

    /// Get parameters from the port.
    pub(crate) fn param_flags(&self) -> &BTreeMap<id::Param, flags::ParamFlag> {
        &self.flags
    }

    /// Set a parameter flag.
    fn set_flag(&mut self, id: id::Param, flag: flags::ParamFlag) {
        match self.flags.entry(id) {
            Entry::Vacant(e) => {
                e.insert(flag);
            }
            Entry::Occupied(e) => {
                if e.get().contains(flag) {
                    return;
                }

                *e.into_mut() |= flag;
            }
        }

        self.modified = true;
    }
}

impl fmt::Debug for Parameters {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parameters")
            .field("param_values", &self.values.keys())
            .field("param_flags", &self.flags)
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
