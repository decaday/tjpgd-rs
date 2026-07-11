#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
pub mod types;
pub mod huffman;
pub mod idct;
pub mod decoder;
pub mod mcu;

pub use error::JpegError;
pub use types::{Rect, PixelFormat, Scale};
pub use decoder::JpegDecoder;
