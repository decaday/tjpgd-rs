use tjpgd_rs::{JpegDecoder, PixelFormat, Rect, Scale};

struct MockReader<'a> {
    data: &'a [u8],
}

impl<'a> embedded_io::ErrorType for MockReader<'a> {
    type Error = embedded_io::ErrorKind;
}

impl<'a> embedded_io::Read for MockReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = buf.len().min(self.data.len());
        if len == 0 { return Ok(0); }
        buf[..len].copy_from_slice(&self.data[..len]);
        self.data = &self.data[len..];
        Ok(len)
    }
}

#[test]
fn test_decoder_initialization_with_invalid_data() {
    let data = [0u8; 128];
    let mut reader = MockReader { data: &data };

    let mut workspace = [0u8; 3100];
    let result = JpegDecoder::new(&mut workspace[..], &mut reader);
    assert!(result.is_err(), "Decoder should fail on invalid JPEG data");
}

#[test]
fn test_types() {
    let rect = Rect {
        left: 0,
        right: 10,
        top: 0,
        bottom: 10,
    };
    assert_eq!(rect.left, 0);

    let format = PixelFormat::RGB565;
    assert_eq!(format, PixelFormat::RGB565);

    let scale = Scale::Half;
    assert_eq!(scale.shift(), 1);
}
