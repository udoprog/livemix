use core::fmt;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use std::collections::HashMap;
use std::io;
use std::os::fd::AsRawFd;
use std::os::fd::OwnedFd;

use anyhow::{Result, bail};
use protocol::flags;
use protocol::id;
use slab::Slab;
use tracing::Level;

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct File {
    file: usize,
    ty: id::DataType,
    fd: OwnedFd,
    flags: flags::MemBlock,
    users: u32,
    region: Option<Region<()>>,
}

#[must_use = "A region must be dropped to release the underlying file descriptor"]
pub(crate) struct Region<T> {
    file: usize,
    pub size: usize,
    pub ptr: NonNull<T>,
}

impl<T> Region<T> {
    /// Read the region.
    #[inline]
    pub unsafe fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { self.ptr.as_ptr().read_volatile() }
    }

    /// Erase the type signature of the region.
    #[inline]
    pub fn erase(self) -> Region<()> {
        Region {
            file: self.file,
            size: self.size,
            ptr: self.ptr.cast(),
        }
    }
}

impl Region<()> {
    /// Slice up the region to a smaller size.
    pub fn slice<T>(&self, offset: isize, size: usize) -> Option<Region<T>> {
        let Ok(offset) = usize::try_from(offset) else {
            return None;
        };

        if offset.checked_add(size)? > self.size {
            return None;
        }

        if mem::size_of::<T>() > 0 {
            assert_eq!(
                mem::size_of::<T>(),
                size,
                "Size of `{}` must match",
                core::any::type_name::<T>()
            );
        }

        Some(Region {
            file: self.file,
            size,
            ptr: unsafe { NonNull::new_unchecked(self.ptr.as_ptr().add(offset).cast()) },
        })
    }

    /// Add an offset to the region's pointer.
    pub fn offset(&self, offset: usize) -> Option<Region<()>> {
        if offset == 0 {
            return Some(Self {
                file: self.file,
                size: self.size,
                ptr: self.ptr,
            });
        }

        let size = self.size.checked_sub(offset)?;

        Some(Region {
            file: self.file,
            size,
            ptr: unsafe { NonNull::new_unchecked(self.ptr.as_ptr().wrapping_add(offset)) },
        })
    }
}

impl<T> fmt::Debug for Region<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Region")
            .field("file", &self.file)
            .field("size", &self.size)
            .field("ptr", &self.ptr)
            .finish()
    }
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
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub(crate) fn insert(
        &mut self,
        mem_id: u32,
        ty: id::DataType,
        fd: OwnedFd,
        flags: flags::MemBlock,
    ) -> Result<usize> {
        if ty != id::DataType::MEM_FD {
            bail!("Memory {mem_id} is not a memfd type, found {ty:?}");
        }

        // If the memory is a file descriptor, get the size of the file
        // since we want to mmap it once.
        let stat = unsafe {
            let mut stat = MaybeUninit::<libc::stat>::uninit();

            if libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr().cast()) == -1 {
                bail!(io::Error::last_os_error());
            }

            stat.assume_init()
        };

        let file = self.files.vacant_key();
        let size = stat.st_size as usize;

        let region = unsafe {
            let mut prot = 0;

            if flags.contains(flags::MemBlock::READABLE) {
                prot |= libc::PROT_READ;
            }

            if flags.contains(flags::MemBlock::WRITABLE) {
                prot |= libc::PROT_WRITE;
            }

            let ptr = libc::mmap(
                std::ptr::null_mut(),
                size,
                prot,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            );

            if ptr.addr().cast_signed() == -1isize {
                bail!(io::Error::last_os_error());
            }

            Region {
                file,
                ptr: NonNull::new_unchecked(ptr.cast()),
                size,
            }
        };

        self.files.insert(File {
            file,
            ty,
            fd,
            flags,
            users: 1,
            region: Some(region),
        });

        if let Some(old) = self.map.insert(mem_id, file) {
            self.free_file(old);
        }

        Ok(file)
    }

    /// Get the data type of a memory region.
    pub(crate) fn data_type(&self, mem_id: u32) -> Option<id::DataType> {
        self.map
            .get(&mem_id)
            .and_then(|&index| self.files.get(index))
            .map(|file| file.ty)
    }

    /// Remove a memory region by its identifier.
    #[tracing::instrument(skip(self))]
    pub(crate) fn remove(&mut self, mem_id: u32) {
        let Some(index) = self.map.remove(&mem_id) else {
            tracing::warn!("Tried to remove memory with id {mem_id} but it was not found");
            return;
        };

        self.free_file(index);
    }

    /// Drop a mapped memory region.
    #[tracing::instrument(skip(self))]
    pub(crate) fn free<T>(&mut self, region: Region<T>) {
        self.free_file(region.file);
    }

    /// Add a user to a memory region.
    pub(crate) fn track<T>(&mut self, region: &Region<T>) {
        if let Some(file) = self.files.get_mut(region.file) {
            file.users += 1;
        }
    }

    /// Map a memory to a region with accessible memory.
    pub(crate) fn map<T>(&mut self, mem_id: u32, offset: isize, size: usize) -> Result<Region<T>> {
        if mem::size_of::<T>() > 0 {
            assert_eq!(
                mem::size_of::<T>(),
                size,
                "Size of `{}` must match",
                core::any::type_name::<T>()
            );
        }

        let Some(file) = self
            .map
            .get_mut(&mem_id)
            .and_then(|&mut index| self.files.get_mut(index))
        else {
            bail!("Memory {mem_id} missing");
        };

        let Some(region) = &file.region else {
            bail!("Memory {mem_id} is not mapped");
        };

        if file.ty != id::DataType::MEM_FD {
            bail!("Memory {mem_id} is not a memfd type, found {:?}", file.ty);
        }

        let Some(region) = region.slice(offset, size) else {
            bail!("Requested offset and size is not valid");
        };

        file.users += 1;
        Ok(region)
    }

    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    fn free_file(&mut self, file: usize) -> bool {
        let Some(fd) = self.files.get_mut(file) else {
            return false;
        };

        fd.users -= 1;

        if fd.users > 0 {
            return false;
        }

        self.files.remove(file);
        true
    }
}
