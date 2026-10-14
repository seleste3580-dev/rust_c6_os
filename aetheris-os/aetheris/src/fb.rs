//! 128x64 1-bit framebuffer in SSD1306 page layout (page-major, bit0 = top row of the page).
use core::convert::Infallible;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

pub const W: i32 = 128;
pub const H: i32 = 64;
pub const BYTES: usize = 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct Fb {
    pub buf: [u8; BYTES],
}

impl Default for Fb {
    fn default() -> Self {
        Self::new()
    }
}

impl Fb {
    pub const fn new() -> Self {
        Self { buf: [0; BYTES] }
    }
    pub fn clear(&mut self) {
        self.buf = [0; BYTES];
    }
    pub fn fill(&mut self) {
        self.buf = [0xFF; BYTES];
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, on: bool) {
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
        let i = ((y as usize) >> 3) * 128 + x as usize;
        let m = 1u8 << (y & 7);
        if on {
            self.buf[i] |= m;
        } else {
            self.buf[i] &= !m;
        }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= W || y >= H {
            return false;
        }
        let i = ((y as usize) >> 3) * 128 + x as usize;
        self.buf[i] & (1u8 << (y & 7)) != 0
    }
    pub fn flip(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
        let i = ((y as usize) >> 3) * 128 + x as usize;
        self.buf[i] ^= 1u8 << (y & 7);
    }
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, on: bool) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, on);
            }
        }
    }
    pub fn invert_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.flip(xx, yy);
            }
        }
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, on: bool) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.hline(x, y, w, on);
        self.hline(x, y + h - 1, w, on);
        self.vline(x, y, h, on);
        self.vline(x + w - 1, y, h, on);
    }
    pub fn hline(&mut self, x: i32, y: i32, w: i32, on: bool) {
        for xx in x..x + w {
            self.set(xx, y, on);
        }
    }
    pub fn vline(&mut self, x: i32, y: i32, h: i32, on: bool) {
        for yy in y..y + h {
            self.set(x, yy, on);
        }
    }
    /// Bresenham line.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, on: bool) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set(x, y, on);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    /// Midpoint circle outline.
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, on: bool) {
        let (mut x, mut y, mut d) = (r, 0, 1 - r);
        while x >= y {
            for (px, py) in [(x, y), (y, x), (-y, x), (-x, y), (-x, -y), (-y, -x), (y, -x), (x, -y)] {
                self.set(cx + px, cy + py, on);
            }
            y += 1;
            if d < 0 {
                d += 2 * y + 1;
            } else {
                x -= 1;
                d += 2 * (y - x) + 1;
            }
        }
    }
    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, on: bool) {
        for yy in -r..=r {
            for xx in -r..=r {
                if xx * xx + yy * yy <= r * r {
                    self.set(cx + xx, cy + yy, on);
                }
            }
        }
    }
    /// Copy rows `y0..y1` of `src` into self (used for per-region wallpapers).
    pub fn copy_from(&mut self, src: &Fb) {
        self.buf = src.buf;
    }
}

impl OriginDimensions for Fb {
    fn size(&self) -> Size {
        Size::new(W as u32, H as u32)
    }
}

impl DrawTarget for Fb {
    type Color = BinaryColor;
    type Error = Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, c) in pixels {
            self.set(p.x, p.y, c == BinaryColor::On);
        }
        Ok(())
    }
}
