use crate::error::JpegError;

/// A simple bump allocator over a mutable byte slice.
pub struct Workspace<'a> {
    pool: &'a mut [u8],
    offset: usize,
}

impl<'a> Workspace<'a> {
    pub fn new(pool: &'a mut [u8]) -> Self {
        Self { pool, offset: 0 }
    }

    /// Allocates `size` bytes aligned to 4 bytes.
    pub fn alloc(&mut self, size: usize) -> Result<&mut [u8], JpegError> {
        // Align size to 4 bytes boundary
        let size = (size + 3) & !3;
        
        if self.offset + size > self.pool.len() {
            return Err(JpegError::InsufficientMemoryPool);
        }

        let start = self.offset;
        self.offset += size;
        Ok(&mut self.pool[start..start + size])
    }

    /// Allocates an array of `T` of length `len`. 
    /// Note: This is highly unsafe in general Rust but safe for primitive types like u8, u16, i32 if we initialize it.
    /// Since we are `#![no_std]` and trying to avoid `alloc`, we'll return a mutable slice of bytes 
    /// and let the caller cast it safely using `bytemuck` or just safe slice operations, or manually initialize.
    pub fn alloc_slice_mut<T: Default + Copy>(&mut self, len: usize) -> Result<&mut [T], JpegError> {
        let bytes_needed = len * core::mem::size_of::<T>();
        let bytes = self.alloc(bytes_needed)?;
        
        // Zero-initialize the slice to safely cast it to T
        bytes.fill(0);

        // SAFETY: We just zero-initialized the memory, and it's aligned to 4 bytes by `alloc` (assuming T's align is <= 4).
        // For larger alignments, we'd need a stricter allocator. For u8/u16/i32/u32, align of 4 is sufficient.
        let ptr = bytes.as_mut_ptr() as *mut T;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        Ok(slice)
    }
}
