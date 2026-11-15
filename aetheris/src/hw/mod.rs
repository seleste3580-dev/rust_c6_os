//! Hardware Abstraction Layer for ESP32-C6.
//!
//! Sub-modules manage: dual-OLED displays (I²C), 4×4 keypad scanning,
//! buzzer (PWM tone), IR sensor, Wi-Fi (esp-radio), persistent flash storage,
//! network stack (embassy-net), and shared inter-task channels.
//!
//! When building from git, replace these stubs with the full source from the repo.

pub mod oled;
pub mod soft_i2c;
pub mod keypad;
pub mod buzzer;
pub mod ir;
pub mod storage;
pub mod net;
pub mod shared;
