//! Text and widget helpers on top of `Fb`.
use crate::fb::Fb;
use alloc::string::String;
use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum F {
    S4x6,
    S5x8,
    M6x10,
    L7x13,
    B7x13,
    XL9x15,
    B9x15,
    H10x20,
}

impl F {
    fn font(self) -> &'static MonoFont<'static> {
        match self {
            F::S4x6 => &FONT_4X6,
            F::S5x8 => &FONT_5X8,
            F::M6x10 => &FONT_6X10,
            F::L7x13 => &FONT_7X13,
            F::B7x13 => &FONT_7X13_BOLD,
            F::XL9x15 => &FONT_9X15,
            F::B9x15 => &FONT_9X15_BOLD,
            F::H10x20 => &FONT_10X20,
        }
    }
    pub fn w(self) -> i32 {
        self.font().character_size.width as i32
    }
    pub fn h(self) -> i32 {
        self.font().character_size.height as i32
    }
}

pub fn text(fb: &mut Fb, x: i32, y: i32, s: &str, f: F, on: bool) {
    let c = if on { BinaryColor::On } else { BinaryColor::Off };
    let st = MonoTextStyle::new(f.font(), c);
    let _ = Text::with_baseline(s, Point::new(x, y), st, Baseline::Top).draw(fb);
}

pub fn text_w(s: &str, f: F) -> i32 {
    s.chars().count() as i32 * f.w()
}

pub fn text_c(fb: &mut Fb, cx: i32, y: i32, s: &str, f: F, on: bool) {
    let w = text_w(s, f);
    text(fb, cx - w / 2, y, s, f, on);
}

pub fn text_r(fb: &mut Fb, rx: i32, y: i32, s: &str, f: F, on: bool) {
    let w = text_w(s, f);
    text(fb, rx - w, y, s, f, on);
}

/// Inverted (white-on-black becomes black-on-white) text with a padded box.
pub fn text_inv(fb: &mut Fb, x: i32, y: i32, s: &str, f: F) {
    let w = text_w(s, f);
    fb.fill_rect(x - 1, y - 1, w + 2, f.h() + 2, true);
    text(fb, x, y, s, f, false);
}

/// Truncate to `n` characters (adds nothing).
pub fn clip(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Draw the standard title band (the yellow 16px strip of the yellow/blue OLED).
pub fn title(fb: &mut Fb, t: &str) {
    fb.fill_rect(0, 0, 128, 16, false);
    text(fb, 2, 1, &clip(t, 10), F::B9x15, true);
    fb.hline(0, 15, 128, true);
}

/// Wi-Fi glyph with 0..=3 bars, 11x8 px.
pub fn wifi_icon(fb: &mut Fb, x: i32, y: i32, bars: u8, connected: bool) {
    for i in 0..3i32 {
        let h = 3 + i * 2;
        let on = (i as u8) < bars;
        if on {
            fb.fill_rect(x + i * 4, y + 8 - h, 3, h, true);
        } else {
            fb.rect(x + i * 4, y + 8 - h, 3, h, true);
        }
    }
    if !connected {
        fb.line(x - 1, y + 9, x + 11, y - 1, true);
    }
}

pub fn speaker_icon(fb: &mut Fb, x: i32, y: i32, vol: u8) {
    fb.fill_rect(x, y + 3, 3, 3, true);
    fb.line(x + 3, y + 3, x + 6, y, true);
    fb.line(x + 3, y + 5, x + 6, y + 8, true);
    fb.vline(x + 6, y, 9, true);
    if vol == 0 {
        fb.line(x + 8, y + 1, x + 12, y + 7, true);
        fb.line(x + 8, y + 7, x + 12, y + 1, true);
    } else {
        fb.set(x + 9, y + 4, true);
        if vol > 2 {
            fb.vline(x + 10, y + 2, 5, true);
        }
        if vol > 4 {
            fb.vline(x + 12, y, 9, true);
        }
    }
}

/// Horizontal progress bar.
pub fn bar(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, num: i32, den: i32) {
    fb.rect(x, y, w, h, true);
    let den = den.max(1);
    let fill = ((w - 4) * num.clamp(0, den)) / den;
    fb.fill_rect(x + 2, y + 2, fill, h - 4, true);
}

/// Footer hint row at the bottom of the main screen (small font).
pub fn hint(fb: &mut Fb, s: &str) {
    fb.fill_rect(0, 55, 128, 9, false);
    fb.hline(0, 54, 128, true);
    text_c(fb, 64, 56, &clip(s, 25), F::S5x8, true);
}

/// Rounded-ish frame.
pub fn frame(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32) {
    fb.rect(x, y, w, h, true);
    fb.set(x, y, false);
    fb.set(x + w - 1, y, false);
    fb.set(x, y + h - 1, false);
    fb.set(x + w - 1, y + h - 1, false);
}
