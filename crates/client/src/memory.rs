//! Types to interact with raw memory.

use core::any;
use core::fmt;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use std::collections::HashMap;
use std::io;
use std::os::fd::AsRawFd;
use std::os::fd::OwnedFd;

use anyhow::ensure;
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

/// A region of memory which is mapped to a file descriptor.
///
/// # Examples
///
/// ```
/// use client::memory::Region;
///
/// let mut data = [0u8; 1024];
///
/// let region = Region::from_slice(0, &mut data[..]);
///
/// assert_eq!(region.size, 1024);
/// assert_eq!(region.ptr.as_ptr(), data.as_mut_ptr().cast());
///
/// let region = region.add(256, 1)?;
///
/// assert_eq!(region.size, 768);
/// assert_eq!(region.ptr.as_ptr(), data.as_mut_ptr().wrapping_add(256).cast());
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use = "A region must be dropped to release the underlying file descriptor"]
#[derive(Clone)]
pub struct Region<T> {
    file: usize,
    pub size: usize,
    pub ptr: NonNull<T>,
}

impl Region<()> {
    /// Cast the region to a different type.
    #[inline]
    pub fn cast<T>(&self) -> Result<Region<T>> {
        ensure!(
            self.ptr.as_ptr().addr() % mem::align_of::<T>() == 0,
            "Region<{}> pointer must be aligned to {}",
            any::type_name::<T>(),
            mem::align_of::<T>()
        );

        ensure!(
            self.size == mem::size_of::<T>(),
            "Region<{}> cast size {} must match {}",
            any::type_name::<T>(),
            mem::size_of::<T>(),
            self.size
        );

        Ok(Region {
            file: self.file,
            size: self.size,
            ptr: self.ptr.cast(),
        })
    }

    /// Limit the size of the region.
    pub fn size(&self, size: usize) -> Result<Region<()>> {
        if size > self.size {
            bail!(
                "Requested size {size} is larger than region size {}",
                self.size
            );
        }

        Ok(Region {
            file: self.file,
            size,
            ptr: self.ptr.cast(),
        })
    }

    /// Add the given size aligned to the specified alignment to the region.
    pub fn add(&self, size: usize, align: usize) -> Result<Region<()>> {
        let size = size.next_multiple_of(align);

        if size > self.size {
            bail!("Offset {size} is larger than region size {}", self.size);
        }

        let ptr = unsafe {
            let ptr = self
                .ptr
                .as_ptr()
                .cast::<u8>()
                .wrapping_add(size)
                .cast::<()>();
            NonNull::new_unchecked(ptr)
        };

        Ok(Region {
            file: self.file,
            size: self.size - size,
            ptr,
        })
    }

    /// Slice up the region to a smaller size.
    pub fn slice<T>(&self, offset: isize, size: usize) -> Result<Region<T>> {
        let Ok(offset) = usize::try_from(offset) else {
            bail!(
                "Region<{}> offset {offset} is not a valid memory offset",
                any::type_name::<T>()
            );
        };

        let Some(offset_size) = offset.checked_add(size) else {
            bail!(
                "Region<{}> offset size {offset} + {size} overflows pointer size",
                any::type_name::<T>()
            );
        };

        ensure!(
            offset_size <= self.size,
            "Region<{}> offset size {offset_size} is larger than region size {}",
            any::type_name::<T>(),
            self.size,
        );

        if mem::size_of::<T>() > 0 {
            ensure!(
                mem::size_of::<T>() == size,
                "Region<{}> is non-zero size {} and must must match {size}",
                any::type_name::<T>(),
                mem::size_of::<T>(),
            );
        }

        Ok(Region {
            file: self.file,
            size,
            ptr: unsafe { NonNull::new_unchecked(self.ptr.as_ptr().add(offset).cast()) },
        })
    }
}

impl<T> Region<T> {
    /// Construct a new region from a slice.
    ///
    /// We require mutable access, all though it won't make a difference for
    /// safety requirements. But it's intended to indicate that whoever
    /// constructs the region at least has the ability to exclusively access it.
    pub fn from_slice(file: usize, data: &mut [T]) -> Self {
        Self {
            file,
            size: data.len(),
            ptr: unsafe { NonNull::new_unchecked(data.as_mut_ptr()).cast() },
        }
    }

    /// Construct a new region.
    pub fn new(file: usize, size: usize, ptr: NonNull<T>) -> Self {
        Self { file, size, ptr }
    }

    /// Read the whole region.
    ///
    /// Since a region might be memory-contested among multiple threads, this
    /// read is never guaranteed to result in data which is up-to-date or event
    /// partially so. We do our best regardless and make use of volatile reads
    /// to try and avoid the reading being optimized away.
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

    /// Coerce the memory region into a reference.
    ///
    /// # Safety
    ///
    /// This is basically never sound, so don't use it for other things than
    /// debugging. The correct way to read the struct is field-wise using the
    /// [`volatile!`] macro.
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// Coerce the memory region into a mutable reference.
    ///
    /// # Safety
    ///
    /// This is basically never sound, so don't use it for other things than
    /// debugging. The correct way to read the struct is field-wise using the
    /// [`volatile!`] macro.
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
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
    #[tracing::instrument(skip(self), ret(level = Level::DEBUG))]
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
                any::type_name::<T>()
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

        let region = region.slice(offset, size)?;
        file.users += 1;
        Ok(region)
    }

    #[tracing::instrument(skip(self), ret(level = Level::DEBUG))]
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
