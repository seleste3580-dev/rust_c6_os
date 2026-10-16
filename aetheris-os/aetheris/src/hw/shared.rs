//! State shared between the UI loop and the background tasks.
use crate::sound::Step;
use crate::sys::{IrSnap, WifiCmd, WifiState};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU8};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

pub enum SoundCmd {
    Seq(&'static [Step]),
    Owned(Vec<Step>),
    Tone(u16),
    Silence,
}

pub static SOUND: Channel<CriticalSectionRawMutex, SoundCmd, 16> = Channel::new();
pub static VOLUME: AtomicU8 = AtomicU8::new(4);

pub static IR: Mutex<CriticalSectionRawMutex, RefCell<IrSnap>> = Mutex::new(RefCell::new(IrSnap::new()));
pub static WIFI_CMD: Channel<CriticalSectionRawMutex, WifiCmd, 8> = Channel::new();
pub static WIFI_STATE: Mutex<CriticalSectionRawMutex, RefCell<WifiState>> = Mutex::new(RefCell::new(WifiState::new()));

/// Encoded settings waiting to be written to flash (latest wins; the storage task debounces).
pub static SAVE: Signal<CriticalSectionRawMutex, Vec<u8>> = Signal::new();

/// Unix time (UTC seconds) obtained via SNTP.
pub static TIME_SYNC: Signal<CriticalSectionRawMutex, i64> = Signal::new();
/// True while the OTA HTTP server should accept connections.
pub static OTA_ON: AtomicBool = AtomicBool::new(false);
pub static PROGRAM_ON: AtomicBool = AtomicBool::new(false);
pub static PROGRAM_TARGET: AtomicU8 = AtomicU8::new(0);
