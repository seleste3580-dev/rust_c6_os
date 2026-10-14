//! Shared UI widgets: multi-tap text prompt with a phone keypad legend on the second screen.
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, TextEntry, tap_chars};
use crate::sys::Sys;
use alloc::string::String;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Res {
    None,
    Submit,
    Cancel,
}

pub struct TextPrompt {
    pub label: String,
    pub entry: TextEntry,
    pub secret: bool,
    pub show: bool,
    last_key: Option<Key>,
    last_t: u64,
}

impl TextPrompt {
    pub fn new(label: &str, max: usize, secret: bool, init: &str) -> Self {
        Self { label: String::from(label), entry: TextEntry::with_text(max, init), secret, show: false, last_key: None, last_t: 0 }
    }
    pub fn text(&self) -> &str {
        &self.entry.text
    }
    /// 0-9 type (multi-tap), * backspace (hold: clear), # OK, A mode, B show/hide, D cancel.
    pub fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Res {
        if ev.long() && ev.key == Key::Star {
            self.entry.clear();
            sys.snd(crate::sound::Snd::Back);
            return Res::None;
        }
        if !ev.press() {
            return Res::None;
        }
        match ev.key {
            Key::Hash => {
                self.entry.commit();
                return Res::Submit;
            }
            Key::D => return Res::Cancel,
            Key::Star => {
                self.entry.backspace();
                sys.snd(crate::sound::Snd::Typing);
            }
            Key::A => {
                self.entry.cycle_mode();
                sys.snd(crate::sound::Snd::Click);
            }
            Key::B => {
                self.show = !self.show;
                sys.snd(crate::sound::Snd::Click);
            }
            Key::C => {
                self.entry.clear();
                sys.snd(crate::sound::Snd::Back);
            }
            k => {
                if self.entry.key(k, sys.now) {
                    self.last_key = Some(k);
                    self.last_t = sys.now;
                    sys.snd(crate::sound::Snd::Typing);
                }
            }
        }
        Res::None
    }
    pub fn tick(&mut self, sys: &Sys) {
        self.entry.tick(sys.now);
    }
    pub fn shown(&self) -> String {
        let n = self.entry.text.chars().count();
        if self.secret && !self.show {
            let mut s: String = core::iter::repeat_n('*', n.saturating_sub(1)).collect();
            if let Some(c) = self.entry.text.chars().last() {
                if self.entry.pending() { s.push(c) } else { s.push('*') }
            }
            s
        } else {
            self.entry.text.clone()
        }
    }
    /// Draw the field on the main screen (title band done by the caller) and legend on the aux screen.
    pub fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::text(a, 3, 19, &gfx::clip(&self.label, 20), F::M6x10, true);
        gfx::frame(a, 2, 31, 124, 15);
        let s = self.shown();
        let n = s.chars().count();
        let vis = 18usize;
        let start = n.saturating_sub(vis);
        let shown: String = s.chars().skip(start).collect();
        gfx::text(a, 5, 34, &shown, F::M6x10, true);
        if (sys.now / 450) % 2 == 0 && !self.entry.pending() {
            let cx = 5 + shown.chars().count() as i32 * 6;
            a.vline(cx, 33, 11, true);
        }
        gfx::hint(a, "#ok *del A:abc B:eye D:x");
        a.fill_rect(104, 19, 22, 9, false);
        gfx::text_r(a, 124, 19, self.entry.mode.label(), F::M6x10, true);
        self.legend(b);
    }
    pub fn legend(&self, b: &mut Fb) {
        let cells: [(Key, &str); 12] = [
            (Key::K1, ".,?"), (Key::K2, "abc"), (Key::K3, "def"),
            (Key::K4, "ghi"), (Key::K5, "jkl"), (Key::K6, "mno"),
            (Key::K7, "pqrs"), (Key::K8, "tuv"), (Key::K9, "wxyz"),
            (Key::Star, "del"), (Key::K0, "spc"), (Key::Hash, "ok"),
        ];
        for (i, (k, lab)) in cells.iter().enumerate() {
            let (c, r) = ((i % 3) as i32, (i / 3) as i32);
            let (x, y) = (c * 43, r * 16);
            let active = self.entry.pending() && Some(*k) == self.last_key;
            if active {
                b.fill_rect(x + 1, y + 1, 41, 14, true);
            } else {
                gfx::frame(b, x + 1, y + 1, 41, 14);
            }
            let mut buf = [0u8; 4];
            let kc = k.ch().encode_utf8(&mut buf);
            gfx::text(b, x + 4, y + 4, kc, F::S5x8, !active);
            let lab: String = if self.entry.mode == crate::input::Mode::Upper {
                lab.to_ascii_uppercase()
            } else if self.entry.mode == crate::input::Mode::Digits && k.digit().is_some() {
                let _ = tap_chars(*k);
                String::from("-")
            } else {
                String::from(*lab)
            };
            gfx::text(b, x + 12, y + 4, &lab, F::S5x8, !active);
        }
    }
}
