use tjpgd3_rs::{JpegDecoder, PixelFormat, Rect, Scale};

#[test]
fn test_decoder_initialization_with_invalid_data() {
    let mut data = [0u8; 128].as_slice();
    let mut read_fn = |buf: &mut [u8]| -> usize {
        let len = buf.len().min(data.len());
        if len == 0 { return 0; }
        buf[..len].copy_from_slice(&data[..len]);
        data = &data[len..];
        len
    };

    let mut workspace = [0u8; 3100];
    let result = JpegDecoder::new(&mut workspace[..], &mut read_fn);
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
