//! Games and visual toys: snake, pong, simon, reaction, life, matrix rain.
use super::{App, Flow, aux_panel};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, Nav};
use crate::sound::Snd;
use crate::sys::Sys;
use crate::wallpaper::Rain;
use alloc::format;
use alloc::vec::Vec;

// ───────────────────────────── Snake ─────────────────────────────
const GW: i32 = 30;
const GH: i32 = 11; // 4px cells in the 48px body => 12 rows, use 11

pub struct Snake {
    body: Vec<(i32, i32)>,
    dir: (i32, i32),
    next_dir: (i32, i32),
    food: (i32, i32),
    next: u64,
    alive: bool,
    started: bool,
    score: u32,
    best: u32,
}

impl Snake {
    pub fn new() -> Self {
        let mut s = Self { body: Vec::new(), dir: (1, 0), next_dir: (1, 0), food: (20, 5), next: 0, alive: true, started: false, score: 0, best: 0 };
        s.reset();
        s
    }
    fn reset(&mut self) {
        self.body = alloc::vec![(6, 5), (5, 5), (4, 5)];
        self.dir = (1, 0);
        self.next_dir = (1, 0);
        self.food = (20, 5);
        self.alive = true;
        self.started = false;
        self.score = 0;
    }
}

impl App for Snake {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        let d = match ev.key.nav() {
            Some(Nav::Up) => (0, -1),
            Some(Nav::Down) => (0, 1),
            Some(Nav::Left) => (-1, 0),
            Some(Nav::Right) => (1, 0),
            Some(Nav::Back) if ev.press() => return Flow::Exit,
            Some(Nav::Ok) if ev.press() => {
                if !self.alive {
                    self.reset();
                }
                self.started = true;
                return Flow::Stay;
            }
            _ => return Flow::Stay,
        };
        if !self.alive {
            return Flow::Stay;
        }
        self.started = true;
        if d.0 != -self.dir.0 || d.1 != -self.dir.1 {
            self.next_dir = d;
        }
        let _ = sys;
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if !self.alive || !self.started || sys.now < self.next {
            return;
        }
        let speed = 190u64.saturating_sub(self.score as u64 * 6).max(70);
        self.next = sys.now + speed;
        self.dir = self.next_dir;
        let h = self.body[0];
        let n = ((h.0 + self.dir.0).rem_euclid(GW), (h.1 + self.dir.1).rem_euclid(GH));
        if self.body.contains(&n) {
            self.alive = false;
            self.best = self.best.max(self.score);
            sys.snd(Snd::Lose);
            return;
        }
        self.body.insert(0, n);
        if n == self.food {
            self.score += 1;
            sys.snd(Snd::Ping);
            sys.led_pulse(80);
            loop {
                let f = (sys.rand_n(GW as u32) as i32, sys.rand_n(GH as u32) as i32);
                if !self.body.contains(&f) {
                    self.food = f;
                    break;
                }
            }
        } else {
            self.body.pop();
        }
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "SNAKE");
        gfx::text_r(a, 125, 3, &format!("{}", self.score), F::B9x15, true);
        a.rect(0, 16, 120, 48, true);
        let ox = 0;
        let oy = 18;
        for (i, &(x, y)) in self.body.iter().enumerate() {
            if i == 0 {
                a.fill_rect(ox + x * 4, oy + y * 4, 4, 4, true);
            } else {
                a.rect(ox + x * 4, oy + y * 4, 4, 4, true);
            }
        }
        a.rect(ox + self.food.0 * 4 + 1, oy + self.food.1 * 4 + 1, 2, 2, true);
        if !self.started {
            gfx::text_inv(a, 22, 34, "press an arrow", F::M6x10);
        }
        if !self.alive {
            gfx::text_inv(a, 8, 34, "GAME OVER # again", F::M6x10);
        }
        aux_panel(b, "SNAKE", &[]);
        gfx::text_c(b, 64, 20, &format!("Score {}", self.score), F::XL9x15, true);
        gfx::text_c(b, 64, 40, &format!("Best {}", self.best), F::M6x10, true);
        gfx::text_c(b, 64, 52, "2468/ABCD steer", F::S5x8, true);
    }
}

// ───────────────────────────── Pong ─────────────────────────────
pub struct Pong {
    py: i32,
    cy: i32,
    bx: i32,
    by: i32,
    vx: i32,
    vy: i32,
    sp: u32,
    sc: (u32, u32),
    next: u64,
    serve: bool,
    held: i32,
}
const PH: i32 = 12;
impl Pong {
    pub fn new() -> Self {
        Self { py: 28, cy: 28, bx: 60, by: 38, vx: 2, vy: 1, sp: 0, sc: (0, 0), next: 0, serve: true, held: 0 }
    }
    fn reset_ball(&mut self, dir: i32) {
        self.bx = 60;
        self.by = 38;
        self.vx = 2 * dir;
        self.vy = 1;
        self.serve = true;
    }
}
impl App for Pong {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        let _ = sys;
        match ev.key.nav() {
            Some(Nav::Up) if ev.down() => self.held = -1,
            Some(Nav::Down) if ev.down() => self.held = 1,
            Some(Nav::Back) if ev.press() => return Flow::Exit,
            Some(Nav::Ok) if ev.press() => self.serve = false,
            _ => {}
        }
        if ev.release() {
            self.held = 0;
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.now < self.next {
            return;
        }
        self.next = sys.now + 40;
        self.py = (self.py + self.held * 3).clamp(17, 63 - PH);
        if self.held != 0 && sys.raw_keys == 0 {
            self.held = 0;
        }
        // cpu paddle
        let target = self.by - PH / 2;
        if self.cy + 1 < target {
            self.cy += 2;
        } else if self.cy - 1 > target {
            self.cy -= 2;
        }
        self.cy = self.cy.clamp(17, 63 - PH);
        if self.serve {
            return;
        }
        self.bx += self.vx;
        self.by += self.vy;
        if self.by <= 17 || self.by >= 61 {
            self.vy = -self.vy;
            self.by = self.by.clamp(17, 61);
            sys.snd(Snd::Click);
        }
        if self.bx <= 6 && self.bx >= 2 && self.by + 2 >= self.py && self.by <= self.py + PH {
            self.vx = self.vx.abs() + if self.sp % 3 == 2 { 1 } else { 0 };
            self.vx = self.vx.min(5);
            self.vy = ((self.by - (self.py + PH / 2)) / 2).clamp(-3, 3);
            self.sp += 1;
            sys.snd(Snd::Select);
        }
        if self.bx >= 117 && self.bx <= 121 && self.by + 2 >= self.cy && self.by <= self.cy + PH {
            self.vx = -self.vx.abs();
            self.vy = ((self.by - (self.cy + PH / 2)) / 2).clamp(-3, 3);
            sys.snd(Snd::Select);
        }
        if self.bx < 0 {
            self.sc.1 += 1;
            sys.snd(Snd::Lose);
            self.reset_ball(1);
        } else if self.bx > 124 {
            self.sc.0 += 1;
            sys.snd(Snd::Win);
            self.reset_ball(-1);
        }
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "PONG");
        gfx::text_r(a, 125, 3, &format!("{}:{}", self.sc.0, self.sc.1), F::B9x15, true);
        for y in (17..63).step_by(6) {
            a.vline(63, y, 3, true);
        }
        a.fill_rect(3, self.py, 3, PH, true);
        a.fill_rect(122, self.cy, 3, PH, true);
        a.fill_rect(self.bx, self.by, 3, 3, true);
        if self.serve {
            gfx::text_inv(a, 22, 40, "# to serve", F::M6x10);
        }
        aux_panel(b, "PONG", &[]);
        gfx::text_c(b, 64, 22, &format!("YOU {}", self.sc.0), F::XL9x15, true);
        gfx::text_c(b, 64, 42, &format!("CPU {}", self.sc.1), F::XL9x15, true);
        gfx::text_c(b, 64, 56, "2/8 paddle  # serve", F::S5x8, true);
    }
}

// ───────────────────────────── Simon ─────────────────────────────
const SIMON_F: [u16; 4] = [415, 310, 252, 209];
pub struct Simon {
    seq: Vec<u8>,
    show: usize,
    input: usize,
    state: u8, // 0 idle, 1 showing, 2 waiting, 3 lost
    next: u64,
    lit: Option<u8>,
    off_at: u64,
    best: usize,
}
impl Simon {
    pub fn new() -> Self {
        Self { seq: Vec::new(), show: 0, input: 0, state: 0, next: 0, lit: None, off_at: 0, best: 0 }
    }
    fn flash(&mut self, q: u8, sys: &mut Sys, ms: u64) {
        self.lit = Some(q);
        self.off_at = sys.now + ms;
        sys.play(alloc::vec![crate::sound::t(SIMON_F[q as usize] * 2, ms as u16)]);
        sys.led_pulse(ms as u16);
    }
}
fn quad(k: Key) -> Option<u8> {
    match k {
        Key::K1 | Key::A => Some(0),
        Key::K3 | Key::D => Some(1),
        Key::K7 | Key::B => Some(2),
        Key::K9 | Key::C => Some(3),
        _ => None,
    }
}
impl App for Simon {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        if ev.key == Key::Star {
            return Flow::Exit;
        }
        if self.state == 0 || self.state == 3 {
            if ev.key == Key::Hash || ev.key == Key::K5 {
                self.seq.clear();
                self.seq.push(sys.rand_n(4) as u8);
                self.state = 1;
                self.show = 0;
                self.next = sys.now + 500;
            }
            return Flow::Stay;
        }
        if self.state == 2
            && let Some(q) = quad(ev.key)
        {
            self.flash(q, sys, 180);
            if q == self.seq[self.input] {
                self.input += 1;
                if self.input == self.seq.len() {
                    self.best = self.best.max(self.seq.len());
                    self.seq.push(sys.rand_n(4) as u8);
                    self.state = 1;
                    self.show = 0;
                    self.next = sys.now + 700;
                    sys.snd(Snd::Accept);
                }
            } else {
                self.state = 3;
                sys.snd(Snd::Error);
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.lit.is_some() && sys.now >= self.off_at {
            self.lit = None;
        }
        if self.state == 1 && sys.now >= self.next {
            if self.show < self.seq.len() {
                let q = self.seq[self.show];
                self.flash(q, sys, 320);
                self.show += 1;
                self.next = sys.now + 520;
            } else {
                self.state = 2;
                self.input = 0;
            }
        }
    }
    fn keep_awake(&self) -> bool {
        self.state != 0
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "SIMON");
        gfx::text_r(a, 125, 3, &format!("{}", self.seq.len().saturating_sub(if self.state == 3 { 1 } else { 0 })), F::B9x15, true);
        let msg = match self.state {
            0 => "# to start",
            1 => "watch...",
            2 => "your turn",
            _ => "wrong! # retry",
        };
        gfx::text_c(a, 64, 28, msg, F::L7x13, true);
        gfx::text_c(a, 64, 46, &format!("best {}", self.best), F::M6x10, true);
        for q in 0..4i32 {
            let (x, y) = (4 + (q % 2) * 62, 2 + (q / 2) * 31);
            if self.lit == Some(q as u8) {
                b.fill_rect(x, y, 58, 28, true);
            } else {
                gfx::frame(b, x, y, 58, 28);
            }
            let lab = ["1/A", "3/D", "7/B", "9/C"][q as usize];
            gfx::text_c(b, x + 29, y + 10, lab, F::M6x10, self.lit != Some(q as u8));
        }
    }
}

// ───────────────────────────── Reaction ─────────────────────────────
pub struct Reaction {
    st: u8, // 0 idle, 1 wait, 2 go, 3 result, 4 early
    at: u64,
    go_at: u64,
    ms: u64,
    best: u64,
}
impl Reaction {
    pub fn new() -> Self {
        Self { st: 0, at: 0, go_at: 0, ms: 0, best: 0 }
    }
}
impl App for Reaction {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        if ev.key == Key::Star {
            return Flow::Exit;
        }
        match self.st {
            0 | 3 | 4 => {
                self.st = 1;
                self.at = sys.now;
                self.go_at = sys.now + 1500 + sys.rand_n(3500) as u64;
                sys.snd(Snd::Click);
            }
            1 => {
                self.st = 4;
                sys.snd(Snd::Error);
            }
            _ => {
                self.ms = sys.now - self.go_at;
                self.best = if self.best == 0 { self.ms } else { self.best.min(self.ms) };
                self.st = 3;
                sys.snd(Snd::Accept);
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.st == 1 && sys.now >= self.go_at {
            self.st = 2;
            sys.play(alloc::vec![crate::sound::t(1800, 120)]);
            sys.led_pulse(400);
        }
    }
    fn keep_awake(&self) -> bool {
        self.st == 1 || self.st == 2
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "REACTION");
        let (m, sub) = match self.st {
            0 => ("READY?", "press any key to arm"),
            1 => ("wait...", "press when screen flashes"),
            2 => ("NOW!", ""),
            3 => ("", "any key: again"),
            _ => ("too early!", "any key: retry"),
        };
        gfx::text_c(a, 64, 24, m, F::XL9x15, true);
        if self.st == 3 {
            gfx::text_c(a, 64, 22, &format!("{} ms", self.ms), F::H10x20, true);
        }
        gfx::text_c(a, 64, 45, sub, F::M6x10, true);
        aux_panel(b, "BEST", &[]);
        gfx::text_c(b, 64, 26, &if self.best == 0 { alloc::string::String::from("--") } else { format!("{} ms", self.best) }, F::H10x20, true);
        if self.st == 2 {
            b.fill();
            gfx::text_c(b, 64, 24, "NOW!", F::H10x20, false);
        }
    }
}

// ───────────────────────────── Matrix ─────────────────────────────
pub struct MatrixApp {
    rain: Rain,
    speed: u32,
    flip: bool,
}
impl MatrixApp {
    pub fn new() -> Self {
        Self { rain: Rain::new(0xC0FFEE), speed: 100, flip: false }
    }
}
impl App for MatrixApp {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.down() {
            match ev.key.nav() {
                Some(Nav::Up) | Some(Nav::Right) => self.speed = (self.speed + 25).min(300),
                Some(Nav::Down) | Some(Nav::Left) => self.speed = self.speed.saturating_sub(25).max(25),
                Some(Nav::Back) => return Flow::Exit,
                Some(Nav::Ok) if ev.press() => {
                    self.flip = !self.flip;
                    sys.snd(Snd::Click);
                }
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, _sys: &mut Sys) {
        self.rain.step(self.speed);
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        self.rain.draw(a);
        if self.flip {
            b.fill();
            self.rain.draw(b);
        } else {
            self.rain.draw(b);
        }
        gfx::text_inv(a, 2, 54, &format!("speed {}%  ^v  # invert", self.speed), F::S5x8);
    }
}

// ───────────────────────────── Game of Life ─────────────────────────────
const LW: usize = 64;
const LH: usize = 32;
pub struct Life {
    g: [[bool; LW]; LH],
    next: u64,
    run: bool,
    generation: u32,
    cx: usize,
    cy: usize,
}
impl Life {
    pub fn new() -> Self {
        let mut l = Self { g: [[false; LW]; LH], next: 0, run: true, generation: 0, cx: 32, cy: 16 };
        for (x, y) in [(10, 5), (11, 6), (9, 7), (10, 7), (11, 7)] {
            l.g[y][x] = true;
        }
        l
    }
    fn step(&mut self) {
        let mut n = [[false; LW]; LH];
        for y in 0..LH {
            for x in 0..LW {
                let mut c = 0;
                for dy in [LH - 1, 0, 1] {
                    for dx in [LW - 1, 0, 1] {
                        if (dx != 0 || dy != 0) && self.g[(y + dy) % LH][(x + dx) % LW] {
                            c += 1;
                        }
                    }
                }
                n[y][x] = matches!((self.g[y][x], c), (true, 2) | (true, 3) | (false, 3));
            }
        }
        self.g = n;
        self.generation += 1;
    }
}
impl App for Life {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        match ev.key {
            Key::K2 => self.cy = (self.cy + LH - 1) % LH,
            Key::K8 => self.cy = (self.cy + 1) % LH,
            Key::K4 => self.cx = (self.cx + LW - 1) % LW,
            Key::K6 => self.cx = (self.cx + 1) % LW,
            Key::K5 if ev.press() => self.g[self.cy][self.cx] = !self.g[self.cy][self.cx],
            Key::Hash if ev.press() => self.run = !self.run,
            Key::A if ev.press() => {
                for y in 0..LH {
                    for x in 0..LW {
                        self.g[y][x] = sys.rand_n(4) == 0;
                    }
                }
                self.generation = 0;
                sys.snd(Snd::Select);
            }
            Key::B if ev.press() => {
                self.g = [[false; LW]; LH];
                self.generation = 0;
            }
            Key::Star | Key::D if ev.press() => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.run && sys.now >= self.next {
            self.next = sys.now + 120;
            self.step();
        }
    }
    fn keep_awake(&self) -> bool {
        self.run
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "LIFE");
        gfx::text_r(a, 125, 3, &format!("g{}", self.generation), F::M6x10, true);
        let pop = self.g.iter().flatten().filter(|&&c| c).count();
        gfx::text(a, 4, 22, &format!("cells {}", pop), F::M6x10, true);
        gfx::text(a, 4, 34, if self.run { "running" } else { "paused" }, F::M6x10, true);
        gfx::hint(a, "5:cell #:run A:rnd B:clr");
        for y in 0..LH {
            for x in 0..LW {
                if self.g[y][x] {
                    b.fill_rect(x as i32 * 2, y as i32 * 2, 2, 2, true);
                }
            }
        }
        if (sys.now / 300) % 2 == 0 {
            b.rect(self.cx as i32 * 2 - 1, self.cy as i32 * 2 - 1, 4, 4, true);
        }
    }
}
