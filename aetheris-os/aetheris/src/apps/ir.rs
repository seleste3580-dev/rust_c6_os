//! IR apps. Works with a 3-pin obstacle/proximity module (LOW when blocked) or a TSOP-type remote receiver.
use super::{App, Flow, aux_panel};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, Nav};
use crate::sound::Snd;
use crate::sys::Sys;
use alloc::format;

pub struct IrMonitor {
    base: u32,
    last_nec: u32,
    flash: u64,
}
impl IrMonitor {
    pub fn new() -> Self {
        Self { base: 0, last_nec: 0, flash: 0 }
    }
}
impl App for IrMonitor {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.press() {
            match ev.key {
                Key::Hash | Key::K5 => {
                    self.base = sys.ir.falls;
                    sys.snd(Snd::Back);
                }
                Key::Star | Key::D => return Flow::Exit,
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.ir.nec_seq != self.last_nec {
            self.last_nec = sys.ir.nec_seq;
            self.flash = sys.now + 400;
            sys.snd(Snd::Ping);
        }
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "IR MONITOR");
        let s = if sys.ir.blocked { "BLOCKED" } else { "CLEAR" };
        gfx::text(a, 4, 20, s, F::XL9x15, true);
        if sys.ir.blocked {
            a.invert_rect(2, 18, 80, 18);
        }
        gfx::text(a, 4, 40, &format!("events {}", sys.ir.falls - self.base), F::M6x10, true);
        gfx::hint(a, "#:reset   *:exit");
        aux_panel(b, "REMOTE / PULSES", &[]);
        match sys.ir.nec {
            Some((ad, cmd)) => {
                let t = format!("A:{:02X} C:{:02X}", ad, cmd);
                gfx::text_c(b, 64, 16, &t, F::XL9x15, true);
                if sys.now < self.flash {
                    b.invert_rect(0, 14, 128, 18);
                }
            }
            None => gfx::text_c(b, 64, 19, "no NEC code yet", F::M6x10, true),
        }
        // raw pulse widths as bars (log-ish scale)
        let n = sys.ir.raw_n.min(16);
        for i in 0..n {
            let w = sys.ir.raw[i] as i32; // units of 100 us
            let h = (w.max(1) as u32).ilog2() as i32 * 3 + 2;
            b.fill_rect(i as i32 * 8, 62 - h.min(26), 6, h.min(26), true);
        }
        gfx::text(b, 2, 34, "pulse widths", F::S5x8, true);
    }
}

pub struct IrCounter {
    base: u32,
    last: u32,
}
impl IrCounter {
    pub fn new() -> Self {
        Self { base: 0, last: 0 }
    }
}
impl App for IrCounter {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.press() {
            match ev.key {
                Key::Hash | Key::K5 => {
                    self.base = sys.ir.falls;
                    sys.snd(Snd::Back);
                }
                Key::Star | Key::D => return Flow::Exit,
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.ir.falls != self.last {
            self.last = sys.ir.falls;
            sys.snd(Snd::Ping);
            sys.led_pulse(80);
        }
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "IR COUNTER");
        let n = sys.ir.falls.saturating_sub(self.base);
        gfx::text_c(a, 64, 22, &format!("{}", n), F::H10x20, true);
        gfx::hint(a, "break the beam  #:reset");
        aux_panel(b, "SENSOR", &[]);
        let r = 18;
        if sys.ir.blocked {
            b.disc(64, 38, r, true);
        } else {
            b.circle(64, 38, r, true);
        }
        gfx::text_c(b, 64, 34, if sys.ir.blocked { "BLOCK" } else { "clear" }, F::M6x10, !sys.ir.blocked);
    }
}

pub struct Tach {
    ppr: u32,
    win_start: u64,
    win_base: u32,
    rpm: u32,
    hist: [u8; 64],
}
impl Tach {
    pub fn new() -> Self {
        Self { ppr: 1, win_start: 0, win_base: 0, rpm: 0, hist: [0; 64] }
    }
}
impl App for Tach {
    fn key(&mut self, ev: KeyEvent, _sys: &mut Sys) -> Flow {
        if ev.down() {
            match ev.key.nav() {
                Some(Nav::Right) | Some(Nav::Up) => self.ppr = (self.ppr + 1).min(12),
                Some(Nav::Left) | Some(Nav::Down) => self.ppr = self.ppr.saturating_sub(1).max(1),
                Some(Nav::Back) if ev.press() => return Flow::Exit,
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.now - self.win_start >= 500 {
            let d = sys.ir.edges.wrapping_sub(self.win_base);
            let dt = (sys.now - self.win_start).max(1) as u32;
            self.rpm = d * 60_000 / dt / self.ppr;
            self.win_start = sys.now;
            self.win_base = sys.ir.edges;
            self.hist.rotate_left(1);
            self.hist[63] = (self.rpm / 100).min(40) as u8;
        }
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "TACHOMETER");
        gfx::text(a, 4, 19, &format!("{}", self.rpm), F::H10x20, true);
        gfx::text(a, 4, 41, &format!("RPM  ({} pulse/rev)", self.ppr), F::M6x10, true);
        gfx::hint(a, "<>:pulses/rev  *:exit");
        aux_panel(b, "RPM HISTORY", &[]);
        for (x, &v) in self.hist.iter().enumerate() {
            b.vline(x as i32 * 2, 60 - v as i32, v as i32 + 1, true);
            b.vline(x as i32 * 2 + 1, 60 - v as i32, v as i32 + 1, true);
        }
    }
}
