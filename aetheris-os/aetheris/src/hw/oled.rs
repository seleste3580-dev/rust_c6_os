//! SSD1306 / SH1106 128x64 OLED driver over any embedded-hal I2C bus (page addressing, dirty-page flush).
use crate::fb::Fb;
use embedded_hal::i2c::I2c;

pub const CONTRAST: [u8; 5] = [0x02, 0x28, 0x70, 0xB0, 0xFF];

pub struct Oled<I: I2c> {
    i2c: I,
    addr: u8,
    col_off: u8,
    last: [u8; 1024],
    valid: bool,
    pub ok: bool,
    pub flip: bool,
}

impl<I: I2c> Oled<I> {
    /// `sh1106`: use the 1.3" SH1106 variant (132-column RAM, +2 column offset).
    pub fn new(i2c: I, addr: u8, sh1106: bool, flip: bool) -> Self {
        Self { i2c, addr, col_off: if sh1106 { 2 } else { 0 }, last: [0; 1024], valid: false, ok: false, flip }
    }
    fn cmds(&mut self, c: &[u8]) -> bool {
        let mut buf = [0u8; 32];
        buf[0] = 0x00;
        let n = c.len().min(31);
        buf[1..=n].copy_from_slice(&c[..n]);
        self.i2c.write(self.addr, &buf[..=n]).is_ok()
    }
    /// Returns false if the panel does not acknowledge (not connected).
    pub fn init(&mut self) -> bool {
        let (remap, com) = if self.flip { (0xA0, 0xC0) } else { (0xA1, 0xC8) };
        let seq: [u8; 24] = [
            0xAE, 0xD5, 0x80, 0xA8, 0x3F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0x02, remap, com, 0xDA, 0x12, 0x81, 0x70, 0xD9, 0xF1, 0xDB,
            0x40, 0xA4, 0xA6,
        ];
        self.ok = self.cmds(&seq);
        if self.ok {
            self.cmds(&[0xAF]);
        }
        self.valid = false;
        self.ok
    }
    pub fn set_contrast(&mut self, level: u8) {
        let c = CONTRAST[(level as usize).min(4)];
        self.cmds(&[0x81, c]);
    }
    pub fn set_invert(&mut self, inv: bool) {
        self.cmds(&[if inv { 0xA7 } else { 0xA6 }]);
    }
    pub fn power(&mut self, on: bool) {
        self.cmds(&[if on { 0xAF } else { 0xAE }]);
        if on {
            self.valid = false;
        }
    }
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
    /// Send only the pages that changed; yields between pages so other tasks keep running.
    pub async fn flush(&mut self, fb: &Fb) {
        if !self.ok {
            return;
        }
        for page in 0..8usize {
            let src = &fb.buf[page * 128..page * 128 + 128];
            if self.valid && self.last[page * 128..page * 128 + 128] == *src {
                continue;
            }
            let co = self.col_off;
            self.cmds(&[0xB0 | page as u8, co & 0x0F, 0x10 | (co >> 4)]);
            let mut buf = [0u8; 129];
            buf[0] = 0x40;
            buf[1..].copy_from_slice(src);
            let _ = self.i2c.write(self.addr, &buf);
            self.last[page * 128..page * 128 + 128].copy_from_slice(src);
            embassy_futures::yield_now().await;
        }
        self.valid = true;
    }
}
