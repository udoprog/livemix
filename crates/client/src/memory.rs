//! Types to interact with raw memory.

use core::any;
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ptr::NonNull;

use core::slice;
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
    region: Option<Region<[MaybeUninit<u8>]>>,
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
/// assert_eq!(region.len(), 1024);
/// assert_eq!(region.as_ptr(), data.as_ptr());
/// # Ok::<_, anyhow::Error>(())
/// ```
#[must_use = "A region must be dropped to release the underlying file descriptor"]
pub struct Region<T>
where
    T: ?Sized,
{
    file: usize,
    size: usize,
    ptr: NonNull<()>,
    _marker: PhantomData<*mut T>,
}

impl Region<[MaybeUninit<u8>]> {
    /// Add the given size aligned to the specified alignment to the region.
    pub fn offset(&self, offset: usize, align: usize) -> Result<Self> {
        let offset = offset.next_multiple_of(align);

        if offset > self.size {
            bail!("Offset {offset} is larger than region size {}", self.size);
        }

        let ptr = unsafe {
            let ptr = self.as_ptr().wrapping_add(offset);
            NonNull::new_unchecked(ptr.cast_mut())
        };

        Ok(Region {
            file: self.file,
            size: self.size - offset,
            ptr: ptr.cast(),
            _marker: PhantomData,
        })
    }
}

impl<T> Region<[T]> {
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
            _marker: PhantomData,
        }
    }

    /// Slice the region to the given offset and size.
    pub fn slice(&self, offset: usize, size: usize) -> Option<Self> {
        if offset + size > self.size {
            return None;
        }

        let ptr = unsafe {
            let ptr = self.as_ptr().wrapping_add(offset);
            NonNull::new_unchecked(ptr.cast_mut())
        };

        Some(Region {
            file: self.file,
            size,
            ptr: ptr.cast(),
            _marker: PhantomData,
        })
    }

    /// Cast the region to a different type.
    #[inline]
    pub fn cast<U>(&self) -> Result<Region<U>> {
        const {
            assert!(mem::size_of::<U>() > 0);
        }

        ensure!(
            self.ptr.as_ptr().addr() % mem::align_of::<U>() == 0,
            "Region<{}> pointer {:p} must be aligned to 0x{:x}",
            any::type_name::<U>(),
            self.ptr.as_ptr(),
            mem::align_of::<T>()
        );

        let size = self.size.wrapping_mul(mem::size_of::<T>());

        ensure!(
            size == mem::size_of::<U>(),
            "Region<{}> cast size {} must match {}",
            any::type_name::<U>(),
            mem::size_of::<U>(),
            size
        );

        Ok(Region {
            file: self.file,
            size: mem::size_of::<U>(),
            ptr: self.ptr.cast(),
            _marker: PhantomData,
        })
    }

    /// Cast the region to a different type.
    #[inline]
    pub fn cast_array<U>(&self) -> Result<Region<[U]>> {
        const {
            assert!(
                mem::size_of::<U>() > 0,
                "Region must be cast to non-zero sized types"
            );
        }

        ensure!(
            self.ptr.as_ptr().addr() % mem::align_of::<U>() == 0,
            "Region<[{}]> pointer must be aligned to {}",
            any::type_name::<U>(),
            mem::align_of::<U>()
        );

        let size = self.size.wrapping_mul(mem::size_of::<T>());

        ensure!(
            size % mem::size_of::<U>() == 0,
            "Region<[{}]> cast array size {} must evenly divide {}",
            any::type_name::<U>(),
            mem::size_of::<U>(),
            size
        );

        let size = size / mem::size_of::<U>();

        Ok(Region {
            file: self.file,
            size,
            ptr: self.ptr.cast(),
            _marker: PhantomData,
        })
    }

    /// Cast the array to a different type.
    ///
    /// The type `U` must be a sized type with the same size as `T`.
    #[inline]
    pub unsafe fn cast_array_unchecked<U>(&self) -> Region<[U]> {
        const {
            assert!(mem::size_of::<U>() == mem::size_of::<T>());
        }

        Region {
            file: self.file,
            size: self.size,
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    /// Limit the size of the region.
    pub fn size(&self, size: usize) -> Result<Region<[T]>> {
        if size > self.size {
            bail!(
                "Requested size {size} is larger than region size {}",
                self.size
            );
        }

        Ok(Region {
            file: self.file,
            size,
            ptr: self.ptr,
            _marker: PhantomData,
        })
    }

    /// Get the length of the region in count of `mem::size_of::<T>()` elements.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Get a pointer to the memory region.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.cast::<T>().as_ptr().cast_const()
    }

    /// Get a mutable pointer to the memory region.
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.cast::<T>().as_ptr()
    }

    /// Coerce the memory region into a reference.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The region is unsafely constructed and if it has the type
        // `[T]` it is assumed to be valid.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.size) }
    }

    /// Coerce the memory region into a mutable reference.
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        // SAFETY: The region is unsafely constructed and if it has the type
        // `[T]` it is assumed to be valid.
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.size) }
    }
}

impl<T> Region<T> {
    /// Construct a new region.
    #[inline]
    pub fn new(file: usize, size: usize, ptr: NonNull<T>) -> Self {
        Self {
            file,
            size,
            ptr: ptr.cast(),
            _marker: PhantomData,
        }
    }

    /// Read the whole region.
    ///
    /// # Safety
    ///
    /// It is up to the caller to ensure that the region is accessed in a
    /// non-contested context where it is exclusively held by the reader.
    ///
    /// Since a region might be memory-contested among multiple threads, this
    /// read is never guaranteed to result in data which is up-to-date or even
    /// partially so. We do our best regardless and make use of volatile reads
    /// to try and avoid the reading being optimized away.
    #[inline]
    pub unsafe fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { self.ptr.cast::<T>().as_ptr().read_volatile() }
    }

    /// Write the whole region.
    ///
    /// # Safety
    ///
    /// It is up to the caller to ensure that the region is accessed in a
    /// non-contested context where it is exclusively held by the reader.
    ///
    /// Since a region might be memory-contested among multiple threads, this
    /// write is never guaranteed to produce data which is visibly up-to-date or
    /// even partially so. We do our best regardless and make use of volatile
    /// reads to try and avoid the reading being optimized away.
    #[inline]
    pub unsafe fn write(&self, value: T)
    where
        T: Copy,
    {
        unsafe {
            self.ptr.cast::<T>().as_ptr().write_volatile(value);
        }
    }

    /// Erase the type signature of the region.
    #[inline]
    pub fn erase(self) -> Region<()> {
        Region {
            file: self.file,
            size: self.size,
            ptr: self.ptr.cast(),
            _marker: PhantomData,
        }
    }

    /// Get a pointer to the memory region.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.cast::<T>().as_ptr().cast_const()
    }

    /// Get a mutable pointer to the memory region.
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.cast::<T>().as_ptr()
    }

    /// Coerce the memory region into a reference.
    ///
    /// # Safety
    ///
    /// This is basically never sound, so don't use it for other things than
    /// debugging. The correct way to read the struct is field-wise using the
    /// [`volatile!`] macro.
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { self.ptr.cast().as_ref() }
    }

    /// Coerce the memory region into a mutable reference.
    ///
    /// # Safety
    ///
    /// This is basically never sound, so don't use it for other things than
    /// debugging. The correct way to read the struct is field-wise using the
    /// [`volatile!`] macro.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.cast().as_mut() }
    }
}

impl<T> Clone for Region<T>
where
    T: ?Sized,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            size: self.size.clone(),
            ptr: self.ptr,
            _marker: self._marker,
        }
    }
}

impl<T> fmt::Debug for Region<T>
where
    T: ?Sized,
{
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
                _marker: PhantomData,
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
    pub(crate) fn free<T>(&mut self, region: Region<T>)
    where
        T: ?Sized,
    {
        self.free_file(region.file);
    }

    /// Add a user to a memory region.
    pub(crate) fn track<T>(&mut self, region: &Region<T>)
    where
        T: ?Sized,
    {
        if let Some(file) = self.files.get_mut(region.file) {
            file.users += 1;
        }
    }

    /// Map a memory to a region with accessible memory.
    pub(crate) fn map(
        &mut self,
        mem_id: u32,
        offset: usize,
        size: usize,
    ) -> Result<Region<[MaybeUninit<u8>]>> {
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

        let region = region.offset(offset, 1)?.size(size)?;
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
