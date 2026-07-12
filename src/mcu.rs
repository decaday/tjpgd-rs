use crate::decoder::JpegDecoder;
use crate::error::JpegError;
use crate::decoder::WriteRect;
use crate::types::{PixelFormat, Rect, Scale};
use crate::idct::{block_idct, ZIG, byteclip};

impl<R: embedded_io::Read, B: core::ops::DerefMut<Target = [u8]>> JpegDecoder<R, B> {
    pub(crate) fn restart(&mut self, rstn: u16) -> Result<(), JpegError> {
        let mut d = 0u16;
        let mut dc = self.dctr;
        let mut dp = self.dptr;

        for _ in 0..2 {
            if dc == 0 {
                let pool_offset = self.inbuf_offset;
                dc = self.reader.read(&mut self.pool[pool_offset..pool_offset + self.inbuf_len]).unwrap_or(0);
                if dc == 0 {
                    return Err(JpegError::InputError);
                }
                dp = pool_offset;
            } else {
                dp += 1;
            }
            dc -= 1;
            d = (d << 8) | (self.pool[dp] as u16);
        }

        self.dptr = dp;
        self.dctr = dc;
        self.dbit = 0;

        if (d & 0xFFD8) != 0xFFD0 || (d & 7) != (rstn & 7) {
            return Err(JpegError::DataFormatError);
        }

        self.dcv = [0; 3];
        Ok(())
    }

    pub(crate) fn mcu_load(&mut self, scale: Scale) -> Result<(), JpegError> {
        let nby = (self.msx * self.msy) as usize;
        
        for blk in 0..nby + 2 {
            let cmp = if blk < nby { 0 } else { blk - nby + 1 };
            
            if cmp > 0 && self.ncomp != 3 {
                // Clear C blocks if not exist
                let mcu_offset = self.mcubuf.offset + blk * 64;
                self.pool[mcu_offset..mcu_offset + 64].fill(128);
            } else {
                let id = if cmp > 0 { 1 } else { 0 };
                
                let d_code = self.huffext(id, 0)?;
                let mut bc = d_code as usize;
                let mut d = self.dcv[cmp] as i32;
                if bc > 0 {
                    let mut e = self.bitext(bc)?;
                    let msb = 1 << (bc - 1);
                    if (e & msb) == 0 {
                        e -= (msb << 1) - 1;
                    }
                    d += e;
                    self.dcv[cmp] = d as i16;
                }
                
                let qtid = self.qtid[cmp] as usize;
                let qt_ref = self.qttbl[qtid];
                let dqf_bytes = &self.pool[qt_ref.offset..qt_ref.offset + qt_ref.len];
                let mut dqf = [0i32; 64];
                dqf.copy_from_slice(bytemuck::cast_slice(dqf_bytes));
                
                let mut tmp = [0i32; 64];
                tmp[0] = (d * dqf[0]) >> 8;
                
                let mut z = 1;
                loop {
                    let d_code = self.huffext(id, 1)?;
                    if d_code == 0 {
                        break;
                    }
                    bc = d_code as usize;
                    z += bc >> 4;
                    if z >= 64 {
                        return Err(JpegError::DataFormatError);
                    }
                    bc &= 0x0F;
                    if bc > 0 {
                        let mut d = self.bitext(bc)?;
                        let msb = 1 << (bc - 1);
                        if (d & msb) == 0 {
                            d -= (msb << 1) - 1;
                        }
                        let i = ZIG[z];
                        tmp[i] = (d * dqf[i]) >> 8;
                    }
                    z += 1;
                    if z >= 64 {
                        break;
                    }
                }
                
                let mcu_offset = self.mcubuf.offset + blk * 64;
                if z == 1 || scale == Scale::Eighth {
                    let fill_val = (tmp[0] / 256 + 128).clamp(0, 255) as u8;
                    self.pool[mcu_offset..mcu_offset + 64].fill(fill_val);
                } else {
                    let mut dst = [0u8; 64];
                    block_idct(&mut tmp, &mut dst);
                    self.pool[mcu_offset..mcu_offset + 64].copy_from_slice(&dst);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn mcu_output<W: WriteRect>(
        &mut self,
        scale: Scale,
        format: PixelFormat,
        writer: &mut W,
        x: u16,
        y: u16,
    ) -> Result<(), JpegError> {
        let mx = (self.msx * 8) as u16;
        let my = (self.msy * 8) as u16;
        
        let mut rx = if x + mx <= self.width { mx } else { self.width - x };
        let mut ry = if y + my <= self.height { my } else { self.height - y };
        
        let shift = scale.shift();
        rx >>= shift;
        ry >>= shift;
        if rx == 0 || ry == 0 {
            return Ok(());
        }
        
        let rect = Rect {
            left: x >> shift,
            right: (x >> shift) + rx - 1,
            top: y >> shift,
            bottom: (y >> shift) + ry - 1,
        };

        // const CVACC: i32 = 1024;
        let mx_scaled = mx >> shift;

        let workbuf_offset = self.workbuf.offset;
        let mcu_offset = self.mcubuf.offset;

        if scale != Scale::Eighth {
            if format != PixelFormat::Grayscale {
                // RGB output
                let mut out_idx = 0;
                for iy in 0..my {
                    let mut pc_idx = mcu_offset;
                    let mut py_idx = mcu_offset;
                    if my == 16 {
                        pc_idx += 64 * 4 + ((iy >> 1) as usize) * 8;
                        if iy >= 8 {
                            py_idx += 64;
                        }
                    } else {
                        pc_idx += (mx as usize) * 8 + (iy as usize) * 8;
                    }
                    py_idx += (iy as usize) * 8;

                    for ix in 0..mx {
                        let cb = (self.pool[pc_idx] as i32) - 128;
                        let cr = (self.pool[pc_idx + 64] as i32) - 128;
                        if mx == 16 {
                            if ix == 8 {
                                py_idx += 64 - 8;
                            }
                            pc_idx += (ix & 1) as usize;
                        } else {
                            pc_idx += 1;
                        }
                        
                        let yy = self.pool[py_idx] as i32;
                        py_idx += 1;

                        self.pool[workbuf_offset + out_idx] = byteclip(yy + ((1435 * cr) >> 10)); out_idx += 1;
                        self.pool[workbuf_offset + out_idx] = byteclip(yy - ((352 * cb + 731 * cr) >> 10)); out_idx += 1;
                        self.pool[workbuf_offset + out_idx] = byteclip(yy + ((1814 * cb) >> 10)); out_idx += 1;
                    }
                }
            } else {
                // Grayscale output
                let mut out_idx = 0;
                for iy in 0..my {
                    let mut py_idx = mcu_offset + (iy as usize) * 8;
                    if my == 16 && iy >= 8 {
                        py_idx += 64;
                    }
                    for ix in 0..mx {
                        if mx == 16 && ix == 8 {
                            py_idx += 64 - 8;
                        }
                        self.pool[workbuf_offset + out_idx] = self.pool[py_idx];
                        py_idx += 1;
                        out_idx += 1;
                    }
                }
            }

            // Descale MCU rectangular if needed
            if scale != Scale::None {
                let s = scale.shift() * 2;
                let w = 1 << scale.shift();
                let a = (mx - w) as usize * if format != PixelFormat::Grayscale { 3 } else { 1 };
                
                let mut op_idx = 0;
                let mut iy = 0;
                while iy < my {
                    let mut ix = 0;
                    while ix < mx {
                        let mut pix_idx = (iy as usize * mx as usize + ix as usize) * if format != PixelFormat::Grayscale { 3 } else { 1 };
                        let mut r = 0u32;
                        let mut g = 0u32;
                        let mut b = 0u32;

                        for _ in 0..w {
                            for _ in 0..w {
                                r += self.pool[workbuf_offset + pix_idx] as u32; pix_idx += 1;
                                if format != PixelFormat::Grayscale {
                                    g += self.pool[workbuf_offset + pix_idx] as u32; pix_idx += 1;
                                    b += self.pool[workbuf_offset + pix_idx] as u32; pix_idx += 1;
                                }
                            }
                            pix_idx += a;
                        }

                        self.pool[workbuf_offset + op_idx] = (r >> s) as u8; op_idx += 1;
                        if format != PixelFormat::Grayscale {
                            self.pool[workbuf_offset + op_idx] = (g >> s) as u8; op_idx += 1;
                            self.pool[workbuf_offset + op_idx] = (b >> s) as u8; op_idx += 1;
                        }
                        
                        ix += w;
                    }
                    iy += w;
                }
            }
        } else {
            // For 1/8 scaling directly
            let mut out_idx = 0;
            let pc_base = mcu_offset + (mx * my) as usize;
            let cb = (self.pool[pc_base] as i32) - 128;
            let cr = (self.pool[pc_base + 64] as i32) - 128;

            let mut iy = 0;
            while iy < my {
                let mut py_idx = mcu_offset;
                if iy == 8 {
                    py_idx += 64 * 2;
                }
                let mut ix = 0;
                while ix < mx {
                    let yy = self.pool[py_idx] as i32;
                    py_idx += 64;

                    if format != PixelFormat::Grayscale {
                        self.pool[workbuf_offset + out_idx] = byteclip(yy + ((1435 * cr) >> 10)); out_idx += 1;
                        self.pool[workbuf_offset + out_idx] = byteclip(yy - ((352 * cb + 731 * cr) >> 10)); out_idx += 1;
                        self.pool[workbuf_offset + out_idx] = byteclip(yy + ((1814 * cb) >> 10)); out_idx += 1;
                    } else {
                        self.pool[workbuf_offset + out_idx] = yy as u8; out_idx += 1;
                    }
                    ix += 8;
                }
                iy += 8;
            }
        }

        // Squeeze up pixel table if truncated
        if rx < mx_scaled {
            let mut s_idx = 0;
            let mut d_idx = 0;
            let bytes_per_pixel = if format != PixelFormat::Grayscale { 3 } else { 1 };
            
            for _y in 0..ry {
                let row_len = rx as usize * bytes_per_pixel;
                self.pool.copy_within(
                    workbuf_offset + s_idx .. workbuf_offset + s_idx + row_len,
                    workbuf_offset + d_idx
                );
                d_idx += row_len;
                s_idx += mx_scaled as usize * bytes_per_pixel;
            }
        }

        let out_len = (rx * ry) as usize;
        let mut final_out_len = out_len * if format == PixelFormat::Grayscale { 1 } else { 3 };

        // Convert RGB888 to RGB565 if needed
        if format == PixelFormat::RGB565 {
            let mut s_idx = 0;
            let mut d_idx = 0;
            for _ in 0..out_len {
                let r = self.pool[workbuf_offset + s_idx]; s_idx += 1;
                let g = self.pool[workbuf_offset + s_idx]; s_idx += 1;
                let b = self.pool[workbuf_offset + s_idx]; s_idx += 1;
                
                let w = ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | (b as u16 >> 3);
                let w_bytes = w.to_be_bytes();
                self.pool[workbuf_offset + d_idx] = w_bytes[0]; d_idx += 1;
                self.pool[workbuf_offset + d_idx] = w_bytes[1]; d_idx += 1;
            }
            final_out_len = out_len * 2;
        }

        if !writer.write_rect(&self.pool[workbuf_offset..workbuf_offset + final_out_len], &rect) {
            return Err(JpegError::Interrupted);
        }

        Ok(())
    }
}
