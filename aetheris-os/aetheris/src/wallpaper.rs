//! Animated wallpapers and the "cmatrix" digital rain engine.
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::trig::*;

pub const NAMES: [&str; 9] = ["Matrix Rain", "Starfield", "Waves", "Retro Grid", "Bubbles", "Seleste", "Circuit", "Radar", "Plain"];
pub const COUNT: u8 = NAMES.len() as u8;

const GLYPHS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ$+-*/=%#&<>[]{}|?!:;~^";
const COLS: usize = 32;
const ROWS: usize = 11;

pub struct Rain {
    y16: [i32; COLS],
    speed: [u8; COLS],
    len: [u8; COLS],
    cells: [[u8; ROWS]; COLS],
    seed: u32,
}

impl Rain {
    pub fn new(seed: u32) -> Self {
        let mut r = Rain { y16: [0; COLS], speed: [0; COLS], len: [0; COLS], cells: [[0; ROWS]; COLS], seed: seed | 1 };
        for c in 0..COLS {
            r.reset_col(c, true);
            for row in 0..ROWS {
                r.cells[c][row] = GLYPHS[(r.rnd() as usize) % GLYPHS.len()];
            }
        }
        r
    }
    fn rnd(&mut self) -> u32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        x
    }
    fn reset_col(&mut self, c: usize, initial: bool) {
        let a = self.rnd();
        self.len[c] = 4 + (a % 6) as u8;
        self.speed[c] = 5 + ((a >> 8) % 12) as u8;
        let start = if initial { -((a >> 16) as i32 % (ROWS as i32 * 16 * 2)) } else { -(self.len[c] as i32 * 16) };
        self.y16[c] = start;
    }
    /// `speed_pct`: 100 = normal.
    pub fn step(&mut self, speed_pct: u32) {
        for c in 0..COLS {
            self.y16[c] += (self.speed[c] as u32 * speed_pct / 100) as i32;
            let head = self.y16[c] / 16;
            if head - self.len[c] as i32 > ROWS as i32 {
                self.reset_col(c, false);
            }
        }
        // flicker a few glyphs per frame
        for _ in 0..6 {
            let a = self.rnd();
            let c = (a as usize) % COLS;
            let r = ((a >> 8) as usize) % ROWS;
            self.cells[c][r] = GLYPHS[((a >> 16) as usize) % GLYPHS.len()];
        }
    }
    pub fn draw(&self, fb: &mut Fb) {
        let mut buf = [0u8; 4];
        for c in 0..COLS {
            let head = self.y16[c].div_euclid(16);
            for r in 0..ROWS as i32 {
                let dist = head - r;
                if dist < 0 || dist >= self.len[c] as i32 {
                    continue;
                }
                let ch = self.cells[c][r as usize] as char;
                let s = ch.encode_utf8(&mut buf);
                let x = c as i32 * 4;
                let y = r * 6;
                if dist == 0 {
                    fb.fill_rect(x, y, 4, 6, true);
                    gfx::text(fb, x, y, s, F::S4x6, false);
                } else {
                    gfx::text(fb, x, y, s, F::S4x6, true);
                    if dist * 3 >= self.len[c] as i32 * 2 {
                        for yy in y..y + 6 {
                            for xx in x..x + 4 {
                                if (xx + yy) & 1 == 1 {
                                    fb.set(xx, yy, false);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct Wall {
    pub kind: u8,
    t: u32,
    rain: Rain,
    stars: [(i16, i16, u8); 36],
    seed: u32,
}

impl Wall {
    pub fn new(kind: u8, seed: u32) -> Self {
        let mut w = Wall { kind, t: 0, rain: Rain::new(seed), stars: [(0, 0, 1); 36], seed: seed | 1 };
        for i in 0..w.stars.len() {
            w.stars[i] = w.new_star(true);
        }
        w
    }
    fn rnd(&mut self) -> u32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        x
    }
    fn new_star(&mut self, anywhere: bool) -> (i16, i16, u8) {
        let a = self.rnd();
        let x = if anywhere { (a % 128) as i16 } else { 127 };
        (x, ((a >> 8) % 64) as i16, 1 + ((a >> 16) % 3) as u8)
    }
    pub fn set_kind(&mut self, kind: u8) {
        self.kind = kind % COUNT;
    }
    pub fn step(&mut self) {
        self.t = self.t.wrapping_add(1);
        match self.kind {
            0 => self.rain.step(100),
            1 => {
                for i in 0..self.stars.len() {
                    let (x, y, s) = self.stars[i];
                    let nx = x - s as i16;
                    self.stars[i] = if nx < 0 { self.new_star(false) } else { (nx, y, s) };
                }
            }
            _ => {}
        }
    }
    pub fn draw(&self, fb: &mut Fb) {
        fb.clear();
        let t = self.t as i32;
        match self.kind {
            0 => self.rain.draw(fb),
            1 => {
                for &(x, y, s) in self.stars.iter() {
                    fb.set(x as i32, y as i32, true);
                    if s >= 2 {
                        fb.set(x as i32 + 1, y as i32, true);
                    }
                    if s == 3 {
                        fb.set(x as i32 + 2, y as i32, true);
                    }
                }
            }
            2 => {
                for k in 0..4 {
                    for x in 0..128 {
                        let y = 28 + k * 9 + sin64(x * 2 + t * (2 + k) + k * 11) * 5 / 127;
                        fb.set(x, y, true);
                    }
                }
            }
            3 => {
                // perspective grid
                fb.hline(0, 24, 128, true);
                for i in -8..=8 {
                    fb.line(64 + i * 4, 24, 64 + i * 22, 63, true);
                }
                let off = (t % 16) as i32;
                for j in 0..6 {
                    let d = ((j * 16 + off) as i32 * 39) / 96;
                    fb.hline(0, 24 + d.min(39), 128, true);
                }
                fb.disc(64, 14, 7, true);
                for k in 0..4 {
                    fb.hline(57, 13 + k * 2, 14, false);
                }
            }
            4 => {
                for i in 0..9i32 {
                    let x = 8 + i * 15 + sin64(t + i * 9) / 24;
                    let y = 63 - ((t * (1 + i % 3) + i * 23) % 70);
                    fb.circle(x, y, 2 + (i % 3), true);
                }
            }
            5 => {
                gfx::text_c(fb, 64, 14, "SELESTE", F::H10x20, true);
                gfx::text_c(fb, 64, 38, "TECHNOLOGIES", F::M6x10, true);
                for i in 0..3i32 {
                    let a = t * (2 + i) + i * 21;
                    fb.set(64 + cos64(a) * (50 - i * 4) / 127, 32 + sin64(a) * (20 - i * 2) / 127, true);
                }
            }
            6 => {
                // circuit traces with travelling pulses
                for i in 0..5i32 {
                    let y = 8 + i * 12;
                    fb.hline(0, y, 128, true);
                    for j in 0..4i32 {
                        let x = 14 + j * 34 + (i * 7) % 20;
                        fb.vline(x, y, 12, true);
                        fb.rect(x - 2, y + 4, 5, 5, true);
                    }
                    let px = (t * 3 + i * 37) % 128;
                    fb.fill_rect(px, y - 1, 3, 3, true);
                }
            }
            7 => {
                fb.circle(64, 32, 30, true);
                fb.circle(64, 32, 20, true);
                fb.circle(64, 32, 10, true);
                fb.hline(34, 32, 60, true);
                fb.vline(64, 2, 60, true);
                let a = t * 2;
                fb.line(64, 32, 64 + cos64(a - 16) * 30 / 127, 32 + sin64(a - 16) * 30 / 127, true);
                for k in 1..6 {
                    let b = a - k * 3;
                    fb.set(64 + cos64(b - 16) * (30 - k) / 127, 32 + sin64(b - 16) * (30 - k) / 127, true);
                }
            }
            _ => {}
        }
    }
}
