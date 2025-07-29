use std::collections::HashMap;
use std::ffi::c_void;
use std::io;
use std::os::fd::AsRawFd;
use std::os::fd::OwnedFd;
use std::ptr::NonNull;

use anyhow::{Result, bail};
use protocol::id;
use slab::Slab;

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct File {
    index: usize,
    ty: id::DataType,
    fd: OwnedFd,
    flags: i32,
    users: u32,
}

#[must_use = "A region must be dropped to release the underlying file descriptor"]
#[derive(Debug)]
pub(crate) struct Region {
    index: usize,
    size: usize,
    ptr: NonNull<c_void>,
}

#[derive(Debug)]
pub(crate) struct Memory {
    map: HashMap<u32, usize>,
    files: Slab<File>,
}

impl Memory {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            files: Slab::new(),
        }
    }

    /// Insert memory.
    #[tracing::instrument(skip(self))]
    pub(crate) fn insert(
        &mut self,
        mem_id: u32,
        ty: id::DataType,
        fd: OwnedFd,
        flags: i32,
    ) -> Result<usize> {
        let index = self.files.vacant_key();

        self.files.insert(File {
            index,
            ty,
            fd,
            flags,
            users: 1,
        });

        if let Some(old) = self.map.insert(mem_id, index) {
            self.free(old);
        }

        tracing::info!(index, "inserted");
        Ok(index)
    }

    /// Get the data type of a memory region.
    pub(crate) fn data_type(&self, mem_id: u32) -> Option<id::DataType> {
        self.map
            .get(&mem_id)
            .and_then(|&index| self.files.get(index))
            .map(|file| file.ty)
    }

    /// Remove a memory region by its identifier.
    pub(crate) fn remove(&mut self, mem_id: u32) {
        let Some(index) = self.map.remove(&mem_id) else {
            tracing::warn!("Tried to remove memory with id {mem_id} but it was not found");
            return;
        };

        self.free(index);
    }

    /// Drop a mapped memory region.
    #[tracing::instrument(skip(self))]
    pub(crate) fn drop(&mut self, region: Region) {
        tracing::info!("dropping region");
        self.free(region.index);
    }

    /// Map a memory to a region with accessible memory.
    pub(crate) fn map(&mut self, mem_id: u32, size: usize, offset: isize) -> Result<Region> {
        let Some(file) = self
            .map
            .get_mut(&mem_id)
            .and_then(|&mut index| self.files.get_mut(index))
        else {
            bail!("Missing memory with identifier {mem_id}");
        };

        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.fd.as_raw_fd(),
                offset as libc::off_t,
            );

            if ptr.addr().cast_signed() == -1isize {
                bail!(io::Error::last_os_error());
            }

            file.users += 1;

            Ok(Region {
                index: file.index,
                ptr: NonNull::new_unchecked(ptr),
                size,
            })
        }
    }

    #[tracing::instrument(skip(self))]
    fn free(&mut self, index: usize) {
        let Some(fd) = self.files.get_mut(index) else {
            return;
        };

        fd.users -= 1;

        if fd.users == 0 {
            tracing::info!("freeing file");
            self.files.remove(index);
        } else {
            tracing::trace!(fd.users, "not freeing file, still in use");
        }
    }
}
