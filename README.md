# tjpgd-rs

`tjpgd-rs` is a `#![no_std]` Rust translation of [TJpgDec (Tiny JPEG Decompressor) R0.03](https://elm-chan.org/fsw/tjpgd/00index.html) by ChaN. 

It is designed for embedded systems with highly constrained memory. It features zero-allocation decoding, meaning it can run entirely using a pre-allocated byte slice (`&mut [u8]`) as its working memory buffer, without requiring a heap allocator.

## Features
- **Zero Allocations**: Fully `#![no_std]` compatible, doesn't require the `alloc` crate.
- **Low Memory Footprint**: Minimal workspace memory required (around 3KB depending on image properties).
- **Fast DCT**: Uses the Arai algorithm for fast 1D IDCT.
- **Multiple Output Formats**: Supports RGB888, RGB565, and Grayscale.
- **Output Scaling**: Supports 1/2, 1/4, and 1/8 descaling during decoding to save memory and processing time.

## Usage

`JpegDecoder` reads input through `embedded_io::Read`. Add `embedded-io` as a direct dependency when implementing a reader:

```toml
[dependencies]
tjpgd-rs = "0.1.0"
embedded-io = { version = "0.7", default-features = false }
```

```rust
use tjpgd_rs::{JpegDecoder, PixelFormat, Scale};

// Provide a reader that implements embedded_io::Read.
struct JpegReader<'a> {
    data: &'a [u8],
}

impl embedded_io::ErrorType for JpegReader<'_> {
    type Error = embedded_io::ErrorKind;
}

impl embedded_io::Read for JpegReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = buf.len().min(self.data.len());
        buf[..len].copy_from_slice(&self.data[..len]);
        self.data = &self.data[len..];
        Ok(len)
    }
}

let reader = JpegReader {
    data: include_bytes!("test.jpg"),
};

// Allocate a workspace buffer (can be on the stack or statically allocated)
// The required size is typically around 3100 bytes for standard JPEGs.
let mut workspace = [0u8; 3500];

// Initialize the decoder (parses headers)
let mut decoder = JpegDecoder::new(&mut workspace[..], reader).unwrap();

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
tjpgd-rs = { version = "0.1.0", features = ["alloc"] }
```

When enabled, you can use `JpegDecoder::new_alloc()`:

```rust
use tjpgd_rs::{JpegDecoder, PixelFormat, Scale};

// Create a JpegReader as shown above.
let reader = JpegReader {
    data: include_bytes!("test.jpg"),
};

// The decoder will automatically allocate a sufficiently large Vec<u8> internally
let mut decoder = JpegDecoder::new_alloc(reader).unwrap();

// Decode exactly as before
decoder.decode(Scale::None, PixelFormat::RGB565, &mut out_fn).unwrap();
```

## Examples

Additional examples, including integrations with display drivers, are available in the [display-driver examples directory](https://github.com/decaday/display-driver/tree/master/examples).

## API Changes from Original C Version
- Removed compile-time macros (`JD_FORMAT`, `JD_USE_SCALE`, `JD_SZBUF`). Formats and scaling are now passed as enum arguments to the `decode` function.
- The `void* pool` has been replaced with a safe `&mut [u8]` slice for the workspace.
- Replaced function pointers with Rust closures (`FnMut`).
- Returns `Result<T, JpegError>` instead of `JRESULT` error codes.

## Development and Contributions

This crate is a Rust translation of [TJpgDec](https://elm-chan.org/fsw/tjpgd/00index.html). Maintenance and contributions therefore focus on translation fidelity and quality, including performance improvements introduced by the translation and the design of the Rust API.

Changes to the underlying JPEG algorithm or requests for new decoder functionality are outside the project's scope.

## License

This project under MIT License.

This project is a direct translation of the C source code of [TJpgDec R0.03](https://elm-chan.org/fsw/tjpgd/00index.html). All credit for the original algorithm and design goes to ChaN.

### Original TJpgDec License & Copyright:

```
/*----------------------------------------------------------------------------/
/ TJpgDec - Tiny JPEG Decompressor R0.xx                       (C)ChaN, 20xx
/-----------------------------------------------------------------------------/
/ The TJpgDec is a generic JPEG decompressor module for tiny embedded systems.
/ This is a free software that opened for education, research and commercial
/ developments under license policy of following terms.
/
/  Copyright (C) 20xx, ChaN, all right reserved.
/
/ * The TJpgDec module is a free software and there is NO WARRANTY.
/ * No restriction on use. You can use, modify and redistribute it for
/   personal, non-profit or commercial products UNDER YOUR RESPONSIBILITY.
/ * Redistributions of source code must retain the above copyright notice.
/----------------------------------------------------------------------------*/
```
