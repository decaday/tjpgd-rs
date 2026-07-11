use std::fs::File;
use std::io::Read;
use tjpgd_rs::decoder::JpegDecoder;
use tjpgd_rs::types::{PixelFormat, Rect, Scale};

fn load_image(path: &str) -> Vec<u8> {
    let mut file = File::open(path).expect("Failed to open image file");
    let mut data = Vec::new();
    file.read_to_end(&mut data).expect("Failed to read image file");
    data
}

struct ImageOutput {
    data: Vec<u8>,
    width: u16,
    #[allow(dead_code)]
    height: u16,
    components: usize,
}

impl ImageOutput {
    fn new(width: u16, height: u16, components: usize) -> Self {
        Self {
            data: vec![0; (width as usize * height as usize) * components],
            width,
            height,
            components,
        }
    }

    fn write_rect(&mut self, data: &[u8], rect: &Rect) -> bool {
        let mut src_idx = 0;
        for y in rect.top..=rect.bottom {
            for x in rect.left..=rect.right {
                let dst_idx = (y as usize * self.width as usize + x as usize) * self.components;
                for c in 0..self.components {
                    if dst_idx + c < self.data.len() && src_idx < data.len() {
                        self.data[dst_idx + c] = data[src_idx];
                    }
                    src_idx += 1;
                }
            }
        }
        true
    }
}

struct MockReader<'a> {
    data: &'a [u8],
}

impl<'a> embedded_io::ErrorType for MockReader<'a> {
    type Error = embedded_io::ErrorKind;
}

impl<'a> embedded_io::Read for MockReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = buf.len().min(self.data.len());
        if len == 0 {
            return Ok(0);
        }
        buf[..len].copy_from_slice(&self.data[..len]);
        self.data = &self.data[len..];
        Ok(len)
    }
}

#[test]
fn test_decode_rustacean_orig_rgb888() {
    let data = load_image("tests/assets/rustacean-orig.jpg");
    let mut reader = MockReader { data: &data };
    let mut workspace = vec![0u8; 35000];
    
    let mut decoder = JpegDecoder::new(&mut workspace[..], &mut reader).expect("Failed to initialize decoder");
    
    let mut output = ImageOutput::new(460, 307, 3);
    let mut writer = |data: &[u8], rect: &Rect| output.write_rect(data, rect);
    
    decoder.decode(Scale::None, PixelFormat::RGB888, &mut writer).expect("Decoding failed");
}

#[test]
fn test_decode_rustacean_orig_scaled_half() {
    let data = load_image("tests/assets/rustacean-orig.jpg");
    let mut reader = MockReader { data: &data };
    let mut workspace = vec![0u8; 35000];
    
    let mut decoder = JpegDecoder::new(&mut workspace[..], &mut reader).unwrap();
    let mut output = ImageOutput::new(460 / 2, 307 / 2, 3);
    let mut writer = |data: &[u8], rect: &Rect| output.write_rect(data, rect);
    
    decoder.decode(Scale::Half, PixelFormat::RGB888, &mut writer).expect("Decoding scaled half failed");
}

#[test]
fn test_decode_rustacean_flat_rgb565() {
    let data = load_image("tests/assets/rustacean-flat.jpg");
    let mut reader = MockReader { data: &data };
    let mut workspace = vec![0u8; 35000];
    
    let mut decoder = JpegDecoder::new(&mut workspace[..], &mut reader).unwrap();
    
    let mut output = ImageOutput::new(460, 307, 2);
    let mut writer = |data: &[u8], rect: &Rect| output.write_rect(data, rect);
    
    decoder.decode(Scale::None, PixelFormat::RGB565, &mut writer).expect("Decoding flat image RGB565 failed");
}

#[test]
fn test_decode_rustacean_orig_grayscale() {
    let data = load_image("tests/assets/rustacean-orig.jpg");
    let mut reader = MockReader { data: &data };
    let mut workspace = vec![0u8; 35000];
    
    let mut decoder = JpegDecoder::new(&mut workspace[..], &mut reader).unwrap();
    
    let mut output = ImageOutput::new(460, 307, 1);
    let mut writer = |data: &[u8], rect: &Rect| output.write_rect(data, rect);
    
    decoder.decode(Scale::None, PixelFormat::Grayscale, &mut writer).expect("Decoding grayscale failed");
}

#[test]
fn test_decode_rustacean_flat_scaled_eighth() {
    let data = load_image("tests/assets/rustacean-flat.jpg");
    let mut reader = MockReader { data: &data };
    let mut workspace = vec![0u8; 35000];
    
    let mut decoder = JpegDecoder::new(&mut workspace[..], &mut reader).unwrap();
    let mut output = ImageOutput::new(460 / 8, 307 / 8, 3);
    let mut writer = |data: &[u8], rect: &Rect| output.write_rect(data, rect);
    
    decoder.decode(Scale::Eighth, PixelFormat::RGB888, &mut writer).expect("Decoding scaled eighth failed");
}
