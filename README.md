# tjpgd3-rs

`tjpgd3-rs` is a `#![no_std]` Rust translation of [TJpgDec (Tiny JPEG Decompressor) R0.03](https://elm-chan.org/fsw/tjpgd/00index.html) by ChaN. 

It is designed for embedded systems with highly constrained memory. It features zero-allocation decoding, meaning it can run entirely using a pre-allocated byte slice (`&mut [u8]`) as its `workspace`, without requiring a heap allocator.

## Features
- **Zero Allocations**: Fully `#![no_std]` compatible, doesn't require the `alloc` crate.
- **Low Memory Footprint**: Minimal workspace memory required (around 3KB depending on image properties).
- **Fast DCT**: Uses the Arai algorithm for fast 1D IDCT.
- **Multiple Output Formats**: Supports RGB888, RGB565, and Grayscale.
- **Output Scaling**: Supports 1/2, 1/4, and 1/8 descaling during decoding to save memory and processing time.

## Usage

```rust
use tjpgd_rs::{JpegDecoder, PixelFormat, Scale};

// Provide a read closure that fills the buffer with JPEG data
let mut image_data = include_bytes!("test.jpg").as_slice();
let mut read_fn = |buf: &mut [u8]| -> usize {
    let len = buf.len().min(image_data.len());
    buf[..len].copy_from_slice(&image_data[..len]);
    image_data = &image_data[len..];
    len
};

// Allocate a workspace buffer (can be on the stack or statically allocated)
// The required size is typically around 3100 bytes for standard JPEGs.
let mut workspace = [0u8; 3500];

// Initialize the decoder (parses headers)
let mut decoder = JpegDecoder::new(&mut workspace[..], &mut read_fn).unwrap();

// Define a closure to process output MCU blocks
let mut out_fn = |data: &[u8], rect: &tjpgd_rs::Rect| -> bool {
    // Write data to the framebuffer at the coordinates specified by rect
    // Return true to continue decoding, false to abort
    true
};

// Start decoding
decoder.decode(Scale::None, PixelFormat::RGB565, &mut out_fn).unwrap();
```

### Using the `alloc` Feature

If you are on a system that supports a global allocator (e.g., you have the `alloc` crate), you can enable the `alloc` feature to avoid manually passing a workspace buffer:

```toml
[dependencies]
tjpgd3-rs = { version = "0.1.0", features = ["alloc"] }
```

When enabled, you can use `JpegDecoder::new_alloc()`:

```rust
use tjpgd3_rs::{JpegDecoder, PixelFormat, Scale};

// ... set up read_fn as above ...

// The decoder will automatically allocate a sufficiently large Vec<u8> internally
let mut decoder = JpegDecoder::new_alloc(&mut read_fn).unwrap();

// Decode exactly as before
decoder.decode(Scale::None, PixelFormat::RGB565, &mut out_fn).unwrap();
```

## API Changes from Original C Version
- Removed compile-time macros (`JD_FORMAT`, `JD_USE_SCALE`, `JD_SZBUF`). Formats and scaling are now passed as enum arguments to the `decode` function.
- The `void* pool` has been replaced with a safe `&mut [u8]` slice for the workspace.
- Replaced function pointers with Rust closures (`FnMut`).
- Returns `Result<T, JpegError>` instead of `JRESULT` error codes.

## License

Dual-licensed under MIT or Apache-2.0.

This project is a direct translation of the C source code of TJpgDec R0.03. All credit for the original algorithm and design goes to ChaN.
