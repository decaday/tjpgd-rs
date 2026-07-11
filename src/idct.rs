#![allow(clippy::erasing_op, clippy::identity_op, clippy::approx_constant)]

/// Zigzag-order to raster-order conversion table
pub const ZIG: [usize; 64] = [
    0,  1,  8, 16,  9,  2,  3, 10, 17, 24, 32, 25, 18, 11,  4,  5,
   12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13,  6,  7, 14, 21, 28,
   35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51,
   58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63
];

/// Input scale factor of Arai algorithm (scaled up 13 bits for fixed point operations)
/// Wait, original says "scaled up 16 bits", but uses 8192 (which is 13 bits).
pub const IPSF: [u16; 64] = [
    (1.00000 * 8192.0) as u16, (1.38704 * 8192.0) as u16, (1.30656 * 8192.0) as u16, (1.17588 * 8192.0) as u16, (1.00000 * 8192.0) as u16, (0.78570 * 8192.0) as u16, (0.54120 * 8192.0) as u16, (0.27590 * 8192.0) as u16,
    (1.38704 * 8192.0) as u16, (1.92388 * 8192.0) as u16, (1.81226 * 8192.0) as u16, (1.63099 * 8192.0) as u16, (1.38704 * 8192.0) as u16, (1.08979 * 8192.0) as u16, (0.75066 * 8192.0) as u16, (0.38268 * 8192.0) as u16,
    (1.30656 * 8192.0) as u16, (1.81226 * 8192.0) as u16, (1.70711 * 8192.0) as u16, (1.53636 * 8192.0) as u16, (1.30656 * 8192.0) as u16, (1.02656 * 8192.0) as u16, (0.70711 * 8192.0) as u16, (0.36048 * 8192.0) as u16,
    (1.17588 * 8192.0) as u16, (1.63099 * 8192.0) as u16, (1.53636 * 8192.0) as u16, (1.38268 * 8192.0) as u16, (1.17588 * 8192.0) as u16, (0.92388 * 8192.0) as u16, (0.63638 * 8192.0) as u16, (0.32442 * 8192.0) as u16,
    (1.00000 * 8192.0) as u16, (1.38704 * 8192.0) as u16, (1.30656 * 8192.0) as u16, (1.17588 * 8192.0) as u16, (1.00000 * 8192.0) as u16, (0.78570 * 8192.0) as u16, (0.54120 * 8192.0) as u16, (0.27590 * 8192.0) as u16,
    (0.78570 * 8192.0) as u16, (1.08979 * 8192.0) as u16, (1.02656 * 8192.0) as u16, (0.92388 * 8192.0) as u16, (0.78570 * 8192.0) as u16, (0.61732 * 8192.0) as u16, (0.42522 * 8192.0) as u16, (0.21677 * 8192.0) as u16,
    (0.54120 * 8192.0) as u16, (0.75066 * 8192.0) as u16, (0.70711 * 8192.0) as u16, (0.63638 * 8192.0) as u16, (0.54120 * 8192.0) as u16, (0.42522 * 8192.0) as u16, (0.29290 * 8192.0) as u16, (0.14932 * 8192.0) as u16,
    (0.27590 * 8192.0) as u16, (0.38268 * 8192.0) as u16, (0.36048 * 8192.0) as u16, (0.32442 * 8192.0) as u16, (0.27590 * 8192.0) as u16, (0.21678 * 8192.0) as u16, (0.14932 * 8192.0) as u16, (0.07612 * 8192.0) as u16,
];

#[inline(always)]
pub fn byteclip(val: i32) -> u8 {
    val.clamp(0, 255) as u8
}

/// Apply Inverse-DCT in Arai Algorithm
pub fn block_idct(src: &mut [i32; 64], dst: &mut [u8]) {
    const M13: i32 = (1.41421 * 4096.0) as i32;
    const M2: i32 = (1.08239 * 4096.0) as i32;
    const M4: i32 = (2.61313 * 4096.0) as i32;
    const M5: i32 = (1.84776 * 4096.0) as i32;

    // Process columns
    for i in 0..8 {
        let mut v0 = src[8 * 0 + i];
        let mut v1 = src[8 * 2 + i];
        let mut v2 = src[8 * 4 + i];
        let mut v3 = src[8 * 6 + i];

        let mut t10 = v0 + v2;
        let mut t12 = v0 - v2;
        let mut t11 = ((v1 - v3) * M13) >> 12;
        v3 += v1;
        t11 -= v3;
        v0 = t10 + v3;
        v3 = t10 - v3;
        v1 = t11 + t12;
        v2 = t12 - t11;

        let mut v4 = src[8 * 7 + i];
        let mut v5 = src[8 * 1 + i];
        let mut v6 = src[8 * 5 + i];
        let mut v7 = src[8 * 3 + i];

        t10 = v5 - v4;
        t11 = v5 + v4;
        t12 = v6 - v7;
        v7 += v6;
        v5 = ((t11 - v7) * M13) >> 12;
        v7 += t11;
        let t13 = ((t10 + t12) * M5) >> 12;
        v4 = t13 - ((t10 * M2) >> 12);
        v6 = t13 - ((t12 * M4) >> 12) - v7;
        v5 -= v6;
        v4 -= v5;

        src[8 * 0 + i] = v0 + v7;
        src[8 * 7 + i] = v0 - v7;
        src[8 * 1 + i] = v1 + v6;
        src[8 * 6 + i] = v1 - v6;
        src[8 * 2 + i] = v2 + v5;
        src[8 * 5 + i] = v2 - v5;
        src[8 * 3 + i] = v3 + v4;
        src[8 * 4 + i] = v3 - v4;
    }

    // Process rows
    for i in 0..8 {
        let base = i * 8;
        let mut v0 = src[base + 0] + (128 << 8); // Remove DC offset here
        let mut v1 = src[base + 2];
        let mut v2 = src[base + 4];
        let mut v3 = src[base + 6];

        let mut t10 = v0 + v2;
        let mut t12 = v0 - v2;
        let mut t11 = ((v1 - v3) * M13) >> 12;
        v3 += v1;
        t11 -= v3;
        v0 = t10 + v3;
        v3 = t10 - v3;
        v1 = t11 + t12;
        v2 = t12 - t11;

        let mut v4 = src[base + 7];
        let mut v5 = src[base + 1];
        let mut v6 = src[base + 5];
        let mut v7 = src[base + 3];

        t10 = v5 - v4;
        t11 = v5 + v4;
        t12 = v6 - v7;
        v7 += v6;
        v5 = ((t11 - v7) * M13) >> 12;
        v7 += t11;
        let t13 = ((t10 + t12) * M5) >> 12;
        v4 = t13 - ((t10 * M2) >> 12);
        v6 = t13 - ((t12 * M4) >> 12) - v7;
        v5 -= v6;
        v4 -= v5;

        // Descale the transformed values 8 bits and output a row
        dst[base + 0] = byteclip((v0 + v7) >> 8);
        dst[base + 7] = byteclip((v0 - v7) >> 8);
        dst[base + 1] = byteclip((v1 + v6) >> 8);
        dst[base + 6] = byteclip((v1 - v6) >> 8);
        dst[base + 2] = byteclip((v2 + v5) >> 8);
        dst[base + 5] = byteclip((v2 - v5) >> 8);
        dst[base + 3] = byteclip((v3 + v4) >> 8);
        dst[base + 4] = byteclip((v3 - v4) >> 8);
    }
}
