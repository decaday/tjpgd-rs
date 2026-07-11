/// Rectangular region in the output image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left end
    pub left: u16,
    /// Right end
    pub right: u16,
    /// Top end
    pub top: u16,
    /// Bottom end
    pub bottom: u16,
}

/// Output pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGB888 (24-bit/pix)
    RGB888,
    /// RGB565 (16-bit/pix)
    RGB565,
    /// Grayscale (8-bit/pix)
    Grayscale,
}

/// Output scaling ratio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// 1/1
    None = 0,
    /// 1/2
    Half = 1,
    /// 1/4
    Quarter = 2,
    /// 1/8
    Eighth = 3,
}

impl Scale {
    pub fn shift(&self) -> u8 {
        *self as u8
    }
}
