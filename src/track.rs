use core::ffi::c_void;
use std::fmt::Debug;
use heapless::FnvIndexMap as IndexMap;
use heapless::FnvIndexSet as IndexSet;
use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};
use core::fmt::{Formatter, Result as FmtResult};
use libc::{MAP_PRIVATE, MAP_ANONYMOUS, mmap};
use crate::page_size;

use super::mem::{align_up_to_page_size, align_down_to_page_size};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u32 {
        const NONE    = PROT_NONE as u32;  // 0
        const READ    = PROT_READ as u32;  // 1
        const WRITE   = PROT_WRITE as u32;  // 2
        const EXECUTE = PROT_EXEC as u32;  // 4
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Block {
    ptr: *mut u8,
    size_in_bytes: usize,
}

impl Block {
    pub fn new(ptr: *mut u8, size_in_bytes: usize) -> Self {
        Self {
            ptr,
            size_in_bytes,
        }
    }

    pub fn page_of(ptr: *mut u8) -> Self {
        let page_size = crate::page_size();
        let start = align_down_to_page_size(ptr as usize, page_size);
        Self {
            ptr: start as *mut u8,
            size_in_bytes: page_size,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.ptr as *const u8, self.size_in_bytes)
        }
    }

    pub fn contains(&self, ptr: *const u8) -> bool {
        let start = self.ptr as usize;
        let end = start + self.size_in_bytes;
        let ptr = ptr as usize;
        ptr >= start && ptr < end
    }

    pub fn change_permissions(&self, new_permissions: Permissions) {
        // Use `mmap` to change the permissions of the memory region
        tracing::info!("Changing permissions for {:p} to {:?}", self.ptr, new_permissions.bits());
        unsafe {
            let page_size = crate::page_size();
            let start = align_down_to_page_size(self.ptr as usize, page_size);
            let end = align_up_to_page_size(self.ptr as usize + self.size_in_bytes, page_size);
            let size = end - start;
            tracing::info!("Changing permissions for {:p} to {:?} (size: {})", self.ptr, new_permissions, size);

            let ptr = libc::mprotect(start as *mut c_void, size, new_permissions.bits() as i32);
            if ptr != 0 {
                panic!("Failed to change permissions");
            }
            tracing::info!("Changed permissions for 0x{:08x} to {:?}", start, new_permissions.bits());
        }
    }
}

unsafe impl Send for Block {}
unsafe impl Sync for Block {}

#[derive(Clone, PartialEq)]
pub struct Track<const N: usize> {
    allocations: IndexMap<*const u8, Block, N>,
}

impl<const N: usize> Track<N> {
    pub const fn new() -> Self {
        Self {
            allocations: IndexMap::new(),
        }
    }

    pub fn insert(&mut self, value: Block) -> Result<bool, Block> {
        if let Ok(val) = self.allocations.insert(value.ptr, value) {
            Ok(val.is_some())
        } else {
            Err(value)
        }
    }

    pub fn remove_ptr(&mut self, value: *const u8) -> Result<Block, ()> {
        if let Some(alloc) = self.allocations.remove(&(value as *const u8)) {
            Ok(alloc)
        } else {
            Err(())
        }
    }

    pub fn remove(&mut self, value: Block) -> Result<Block, ()> {
        self.remove_ptr(value.ptr as *const u8)
    }

    pub fn get(&self, ptr: *const u8) -> Option<Block> {
        if let Some(alloc) = self.allocations.get(&ptr).copied() {
            Some(alloc)
        } else {
            // See if the pointer is within the bounds of an allocation
            for (_, alloc) in self.allocations.iter() {
                if alloc.contains(ptr) {
                    return Some(*alloc);
                }
            }
            None
        }
    }

    pub fn get_size(&self, ptr: *const u8) -> Option<usize> {
        self.get(ptr).map(|alloc| alloc.size_in_bytes)
    }
}

unsafe impl<const N: usize> Send for Track<N> {}
unsafe impl<const N: usize> Sync for Track<N> {}

impl<const N: usize> Debug for Track<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Track")
            .field("allocations", &self.allocations)
            .finish()
    }
}