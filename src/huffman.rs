use crate::decoder::JpegDecoder;
use crate::error::JpegError;
use crate::decoder::Read;

impl<R: Read, B: core::ops::DerefMut<Target = [u8]>> JpegDecoder<R, B> {
    pub(crate) fn create_qt_tbl(&mut self, pool_offset: usize, seg_len: usize) -> Result<(), JpegError> {
        let mut data_idx = pool_offset;
        let end_idx = pool_offset + seg_len;
        
        while data_idx < end_idx {
            if end_idx - data_idx < 65 {
                return Err(JpegError::DataFormatError);
            }
            let d = self.pool[data_idx];
            data_idx += 1;
            
            if (d & 0xF0) != 0 {
                return Err(JpegError::DataFormatError); // Only 8-bit resolution
            }
            let i = (d & 3) as usize;
            
            let ofs = self.alloc(64 * 4)?;
            self.qttbl[i] = crate::decoder::TableRef { offset: ofs, len: 64 * 4 };
            
            let mut pb = [0i32; 64];
            for j in 0..64 {
                let zi = crate::idct::ZIG[j];
                // Apply scale factor of Arai algorithm
                pb[zi] = ((self.pool[data_idx + j] as u32) * (crate::idct::IPSF[zi] as u32)) as i32;
            }
            data_idx += 64;

            // Copy to pool
            let dst = &mut self.pool[ofs..ofs + 64 * 4];
            for (idx, &val) in pb.iter().enumerate() {
                dst[idx * 4..idx * 4 + 4].copy_from_slice(&val.to_ne_bytes());
            }
        }
        Ok(())
    }

    pub(crate) fn create_huffman_tbl(&mut self, pool_offset: usize, seg_len: usize) -> Result<(), JpegError> {
        let mut data_idx = pool_offset;
        let end_idx = pool_offset + seg_len;

        while data_idx < end_idx {
            if end_idx - data_idx < 17 {
                return Err(JpegError::DataFormatError);
            }
            let d = self.pool[data_idx];
            data_idx += 1;
            
            if (d & 0xEE) != 0 {
                return Err(JpegError::DataFormatError);
            }
            let cls = (d >> 4) as usize;
            let num = (d & 0x0F) as usize;

            let ofs_bits = self.alloc(16)?;
            self.huffbits[num][cls] = crate::decoder::TableRef { offset: ofs_bits, len: 16 };
            let mut np = 0;
            let mut pb = [0u8; 16];
            for i in 0..16 {
                pb[i] = self.pool[data_idx + i];
                np += pb[i] as usize;
            }
            data_idx += 16;
            
            for i in 0..16 {
                self.pool[ofs_bits + i] = pb[i];
            }

            let ofs_code = self.alloc(np * 2)?;
            self.huffcode[num][cls] = crate::decoder::TableRef { offset: ofs_code, len: np * 2 };
            
            let mut hc = 0u16;
            let mut j = 0;
            let mut code_buf = [0u8; 512]; // Max np is 256 for JPEG
            for i in 0..16 {
                let mut b = pb[i];
                while b > 0 {
                    b -= 1;
                    let bytes = hc.to_ne_bytes();
                    code_buf[j * 2] = bytes[0];
                    code_buf[j * 2 + 1] = bytes[1];
                    j += 1;
                    hc += 1;
                }
                hc <<= 1;
            }
            
            for i in 0..np * 2 {
                self.pool[ofs_code + i] = code_buf[i];
            }

            if end_idx - data_idx < np {
                return Err(JpegError::DataFormatError);
            }
            let ofs_data = self.alloc(np)?;
            self.huffdata[num][cls] = crate::decoder::TableRef { offset: ofs_data, len: np };
            
            for i in 0..np {
                let d = self.pool[data_idx + i];
                if cls == 0 && d > 11 {
                    return Err(JpegError::DataFormatError);
                }
                self.pool[ofs_data + i] = d;
            }
            data_idx += np;
        }
        Ok(())
    }

    pub(crate) fn bitext(&mut self, mut nbit: usize) -> Result<i32, JpegError> {
        let mut dc = self.dctr;
        let mut dp = self.dptr;
        let mut d = 0i32;
        let mut mbit = self.dbit;
        let mut flg = false;

        loop {
            if mbit == 0 {
                if dc == 0 {
                    // Actually we should read directly into pool
                    let pool_offset = self.inbuf_offset;
                    dc = self.reader.read(&mut self.pool[pool_offset..pool_offset + self.inbuf_len]);
                    if dc == 0 {
                        return Err(JpegError::InputError);
                    }
                    dp = pool_offset;
                } else {
                    dp += 1;
                }
                dc -= 1;
                if flg {
                    flg = false;
                    if self.pool[dp] != 0 {
                        return Err(JpegError::DataFormatError);
                    }
                    self.pool[dp] = 0xFF;
                } else {
                    if self.pool[dp] == 0xFF {
                        flg = true;
                        continue;
                    }
                }
                mbit = 0x80;
            }
            d <<= 1;
            if (self.pool[dp] & mbit) != 0 {
                d |= 1;
            }
            mbit >>= 1;
            nbit -= 1;
            if nbit == 0 {
                break;
            }
        }

        self.dbit = mbit;
        self.dctr = dc;
        self.dptr = dp;
        Ok(d)
    }

    pub(crate) fn huffext(&mut self, id: usize, cls: usize) -> Result<u8, JpegError> {
        let mut dc = self.dctr;
        let mut dp = self.dptr;
        let mut d = 0u16;
        let mut flg = false;
        let mut bm = self.dbit;
        let mut bl = 16;

        let huffbits = self.huffbits[id][cls];
        let huffcode = self.huffcode[id][cls];
        let huffdata = self.huffdata[id][cls];

        let mut hb_idx = huffbits.offset;
        let mut hc_idx = huffcode.offset;
        let mut hd_idx = huffdata.offset;

        loop {
            if bm == 0 {
                if dc == 0 {
                    let pool_offset = self.inbuf_offset;
                    dc = self.reader.read(&mut self.pool[pool_offset..pool_offset + self.inbuf_len]);
                    if dc == 0 {
                        return Err(JpegError::InputError);
                    }
                    dp = pool_offset;
                } else {
                    dp += 1;
                }
                dc -= 1;
                if flg {
                    flg = false;
                    if self.pool[dp] != 0 {
                        return Err(JpegError::DataFormatError);
                    }
                    self.pool[dp] = 0xFF;
                } else {
                    if self.pool[dp] == 0xFF {
                        flg = true;
                        continue;
                    }
                }
                bm = 0x80;
            }
            d <<= 1;
            if (self.pool[dp] & bm) != 0 {
                d |= 1;
            }
            bm >>= 1;

            let mut nd = self.pool[hb_idx];
            hb_idx += 1;
            while nd > 0 {
                let hc_val = u16::from_ne_bytes([self.pool[hc_idx], self.pool[hc_idx + 1]]);
                if d == hc_val {
                    self.dbit = bm;
                    self.dctr = dc;
                    self.dptr = dp;
                    return Ok(self.pool[hd_idx]);
                }
                hc_idx += 2;
                hd_idx += 1;
                nd -= 1;
            }
            bl -= 1;
            if bl == 0 {
                break;
            }
        }

        Err(JpegError::DataFormatError)
    }
}
