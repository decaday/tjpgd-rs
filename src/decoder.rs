use crate::error::JpegError;
use crate::types::{PixelFormat, Rect, Scale};


/// Output function trait or closure equivalent.
pub trait WriteRect {
    /// Writes a decoded MCU block.
    /// Returns true to continue, false to abort.
    fn write_rect(&mut self, data: &[u8], rect: &Rect) -> bool;
}

impl<F> WriteRect for F
where
    F: FnMut(&[u8], &Rect) -> bool,
{
    fn write_rect(&mut self, data: &[u8], rect: &Rect) -> bool {
        self(data, rect)
    }
}

#[derive(Default, Clone, Copy)]
pub(crate) struct TableRef {
    pub(crate) offset: usize,
    pub(crate) len: usize,
}

pub struct JpegDecoder<R: embedded_io::Read, B: core::ops::DerefMut<Target = [u8]>> {
    pub(crate) pool: B,
    pub(crate) pool_used: usize,

    pub(crate) reader: R,
    pub(crate) inbuf_offset: usize,
    pub(crate) inbuf_len: usize,

    pub(crate) dctr: usize,
    pub(crate) dptr: usize, // index in inbuf
    pub(crate) dbit: u8,

    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) ncomp: u8,
    pub(crate) msx: u8,
    pub(crate) msy: u8,
    pub(crate) qtid: [u8; 3],
    pub(crate) dcv: [i16; 3],
    pub(crate) nrst: u16,

    // [id][dcac]
    pub(crate) huffbits: [[TableRef; 2]; 2],
    pub(crate) huffcode: [[TableRef; 2]; 2],
    pub(crate) huffdata: [[TableRef; 2]; 2],
    // [id]
    pub(crate) qttbl: [TableRef; 4],

    pub(crate) workbuf: TableRef,
    pub(crate) mcubuf: TableRef,
}

impl<R: embedded_io::Read, B: core::ops::DerefMut<Target = [u8]>> JpegDecoder<R, B> {
    pub fn new(pool: B, reader: R) -> Result<Self, JpegError> {
        let mut jd = Self {
            pool,
            pool_used: 0,
            reader,
            inbuf_offset: 0,
            inbuf_len: 512, // JD_SZBUF
            dctr: 0,
            dptr: 0,
            dbit: 0,
            width: 0,
            height: 0,
            ncomp: 0,
            msx: 0,
            msy: 0,
            qtid: [0; 3],
            dcv: [0; 3],
            nrst: 0,
            huffbits: [[TableRef::default(); 2]; 2],
            huffcode: [[TableRef::default(); 2]; 2],
            huffdata: [[TableRef::default(); 2]; 2],
            qttbl: [TableRef::default(); 4],
            workbuf: TableRef::default(),
            mcubuf: TableRef::default(),
        };

        // Allocate inbuf
        jd.inbuf_offset = jd.alloc(jd.inbuf_len)?;

        // Find SOI marker
        let mut buf = [0u8; 2];
        let ofs = 0;
        loop {
            if jd.reader.read(&mut buf[0..1]).map_err(|_| JpegError::InputError)? != 1 {
                return Err(JpegError::InputError);
            }
            if buf[0] == 0xFF {
                if jd.reader.read(&mut buf[1..2]).map_err(|_| JpegError::InputError)? != 1 {
                    return Err(JpegError::InputError);
                }
                if buf[1] == 0xD8 {
                    break;
                }
            }
        }

        // Parse headers
        jd.parse_headers(ofs)?;

        Ok(jd)
    }

    pub(crate) fn alloc(&mut self, size: usize) -> Result<usize, JpegError> {
        let size = (size + 3) & !3; // align to 4
        if self.pool_used + size > self.pool.len() {
            return Err(JpegError::InsufficientMemoryPool);
        }
        let offset = self.pool_used;
        self.pool_used += size;
        Ok(offset)
    }

    fn parse_headers(&mut self, mut _ofs: usize) -> Result<(), JpegError> {
        let mut buf = [0u8; 2];
        loop {
            let pool_offset = self.inbuf_offset;
            // Read 2 bytes for marker
            if self.reader.read(&mut buf[0..1]).unwrap_or(0) != 1 { return Err(JpegError::InputError); }
            if self.reader.read(&mut buf[1..2]).unwrap_or(0) != 1 { return Err(JpegError::InputError); }
            let marker = ((buf[0] as u16) << 8) | (buf[1] as u16);

            // Read 2 bytes for length
            if self.reader.read(&mut buf[0..1]).unwrap_or(0) != 1 { return Err(JpegError::InputError); }
            if self.reader.read(&mut buf[1..2]).unwrap_or(0) != 1 { return Err(JpegError::InputError); }
            let len = ((buf[0] as u16) << 8) | (buf[1] as u16);

            if len <= 2 || (marker >> 8) != 0xFF {
                return Err(JpegError::DataFormatError);
            }
            let seg_len = (len - 2) as usize;
            _ofs += 4 + seg_len;

            let is_supported = match marker & 0xFF {
                0xC0 | 0xDD | 0xC4 | 0xDB | 0xDA => true,
                0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF | 0xD9 => {
                    return Err(JpegError::UnsupportedJpegStandard);
                }
                _ => false,
            };

            if is_supported {
                // Load segment data into pool inbuf
                if seg_len > self.inbuf_len {
                    return Err(JpegError::InsufficientStreamBuffer);
                }
                
                // Read exactly seg_len bytes
                let mut read_total = 0;
                while read_total < seg_len {
                    let bytes_read = self.reader.read(&mut self.pool[pool_offset + read_total..pool_offset + seg_len]).unwrap_or(0);
                    if bytes_read == 0 {
                        return Err(JpegError::InputError);
                    }
                    read_total += bytes_read;
                }
                match marker & 0xFF {
                    0xC0 => { // SOF0
                        self.height = ((self.pool[pool_offset + 1] as u16) << 8) | (self.pool[pool_offset + 2] as u16);
                        self.width = ((self.pool[pool_offset + 3] as u16) << 8) | (self.pool[pool_offset + 4] as u16);
                        self.ncomp = self.pool[pool_offset + 5];
                        if self.ncomp != 3 && self.ncomp != 1 {
                            return Err(JpegError::UnsupportedJpegStandard);
                        }
                        for i in 0..self.ncomp as usize {
                            let b = self.pool[pool_offset + 7 + 3 * i];
                            if i == 0 {
                                if b != 0x11 && b != 0x22 && b != 0x21 {
                                    return Err(JpegError::UnsupportedJpegStandard);
                                }
                                self.msx = b >> 4;
                                self.msy = b & 15;
                            } else {
                                if b != 0x11 {
                                    return Err(JpegError::UnsupportedJpegStandard);
                                }
                            }
                            self.qtid[i] = self.pool[pool_offset + 8 + 3 * i];
                            if self.qtid[i] > 3 {
                                return Err(JpegError::UnsupportedJpegStandard);
                            }
                        }
                    }
                    0xDD => { // DRI
                        self.nrst = ((self.pool[pool_offset] as u16) << 8) | (self.pool[pool_offset + 1] as u16);
                    }
                    0xC4 => { // DHT
                        self.create_huffman_tbl(pool_offset, seg_len)?;
                    }
                    0xDB => { // DQT
                        self.create_qt_tbl(pool_offset, seg_len)?;
                    }
                    0xDA => { // SOS
                        if self.width == 0 || self.height == 0 {
                            return Err(JpegError::DataFormatError);
                        }
                        if self.pool[pool_offset] != self.ncomp {
                            return Err(JpegError::UnsupportedJpegStandard);
                        }
                        for i in 0..self.ncomp as usize {
                            let b = self.pool[pool_offset + 2 + 2 * i];
                            if b != 0x00 && b != 0x11 {
                                return Err(JpegError::UnsupportedJpegStandard);
                            }
                            let n = if i == 0 { 0 } else { 1 };
                            if self.huffbits[n][0].len == 0 || self.huffbits[n][1].len == 0 {
                                return Err(JpegError::DataFormatError);
                            }
                            if self.qttbl[self.qtid[i] as usize].len == 0 {
                                return Err(JpegError::DataFormatError);
                            }
                        }

                        // Allocate workbuf and mcubuf
                        let n = (self.msy * self.msx) as usize;
                        if n == 0 {
                            return Err(JpegError::DataFormatError);
                        }
                        let mut work_len = n * 64 * 2 + 64;
                        if work_len < 256 {
                            work_len = 256;
                        }
                        let w_ofs = self.alloc(work_len)?;
                        self.workbuf = TableRef { offset: w_ofs, len: work_len };

                        let m_ofs = self.alloc((n + 2) * 64)?;
                        self.mcubuf = TableRef { offset: m_ofs, len: (n + 2) * 64 };

                        self.dctr = 0;
                        self.dptr = 0;

                        return Ok(()); // SOS is the last header
                    }
                    _ => unreachable!(),
                }
            } else {
                // Skip unknown segment
                let mut to_skip = seg_len;
                while to_skip > 0 {
                    let read_len = core::cmp::min(to_skip, self.inbuf_len);
                    let mut tmp = [0u8; 1];
                    let mut bytes_read = 0;
                    for _ in 0..read_len {
                        let b = self.reader.read(&mut tmp).unwrap_or(0);
                        if b == 0 { break; }
                        bytes_read += b;
                    }
                    if bytes_read == 0 {
                        return Err(JpegError::InputError);
                    }
                    to_skip -= bytes_read;
                }
            }
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn decode<W: WriteRect>(
        &mut self,
        scale: Scale,
        format: PixelFormat,
        writer: &mut W,
    ) -> Result<(), JpegError> {
        let mx = (self.msx * 8) as u16;
        let my = (self.msy * 8) as u16;

        self.dcv = [0; 3];
        let mut rst = 0;
        let mut rsc = 0;

        let mut y = 0;
        while y < self.height {
            let mut x = 0;
            while x < self.width {
                if self.nrst > 0 {
                    if rst == self.nrst {
                        self.restart(rsc)?;
                        rsc += 1;
                        rst = 0;
                    }
                    rst += 1;
                }
                self.mcu_load(scale)?;
                self.mcu_output(scale, format, writer, x, y)?;
                x += mx;
            }
            y += my;
        }

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl<R: embedded_io::Read, B: core::ops::DerefMut<Target = [u8]>> JpegDecoder<R, B> {
    /// Creates a new `JpegDecoder` by allocating an owned buffer internally.
    /// This requires the `alloc` feature to be enabled.
    pub fn new_alloc(reader: R) -> Result<JpegDecoder<R, alloc::vec::Vec<u8>>, JpegError> {
        // 3500 bytes is a safe default workspace size for TJpgDec
        let pool = alloc::vec![0u8; 3500];
        JpegDecoder::new(pool, reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyReader;
    impl embedded_io::ErrorType for DummyReader {
        type Error = embedded_io::ErrorKind;
    }
    impl embedded_io::Read for DummyReader {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
            Ok(0)
        }
    }

    #[test]
    fn test_decoder_init_insufficient_pool() {
        let mut reader = DummyReader;
        let mut pool = [0u8; 128]; // Too small, needs >= 3100
        let decoder = JpegDecoder::new(&mut pool[..], &mut reader);
        assert!(matches!(decoder, Err(JpegError::InsufficientMemoryPool)));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_decoder_new_alloc() {
        // Because DummyReader immediately returns EOF, finding SOI marker fails.
        // The memory allocation itself should succeed.
        let reader = DummyReader;
        let decoder = JpegDecoder::<DummyReader, &mut [u8]>::new_alloc(reader);
        assert!(matches!(decoder, Err(JpegError::InputError)));
    }
}
