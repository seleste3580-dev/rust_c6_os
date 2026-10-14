//! Aetheris — a phone-style operating environment for the ESP32-C6, by Seleste Technologies.
//!
//! Everything above `hw` is hardware-independent (and unit-testable on a PC); `hw` contains the drivers.
#![no_std]

extern crate alloc;

pub mod apps;
pub mod clock;
pub mod fb;
pub mod gfx;
#[cfg(target_os = "none")]
pub mod hw;
pub mod icons;
pub mod input;
pub mod ir_decode;
pub mod notes;
pub mod os;
pub mod settings;
pub mod sound;
pub mod sys;
pub mod trig;
pub mod tunes;
pub mod wallpaper;
