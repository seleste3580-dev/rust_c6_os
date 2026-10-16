//! Aetheris OS firmware entry point — Seleste Technologies.
//!
//! generator parameters: esp32c6 · esp-hal 1.2 · esp-rtos (embassy) · esp-radio Wi-Fi · alloc · log
#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

extern crate alloc;

use aetheris::fb::Fb;
use aetheris::hw::buzzer::buzzer_task;
use aetheris::hw::ir::ir_task;
use aetheris::hw::keypad::Keypad;
use aetheris::hw::net::{http_task, net_runner, wifi_task};
use aetheris::hw::oled::Oled;
use aetheris::hw::shared::*;
use aetheris::hw::soft_i2c::SoftI2c;
use aetheris::hw::storage::{self, FLASH, storage_task};
use aetheris::input::{Key, KeyEvent, KeyScanner};
use aetheris::os::{Os, Power};
use aetheris::sound::Snd;
use aetheris::sys::{Effect, Info, Sys, WifiStatus};
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig, Pin};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::rng::Rng;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{Config as UartConfig, Uart};
use esp_radio::wifi::{Interface, WifiController};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

/// OLED panel variants: set `true` for 1.3" SH1106 modules, `false` for 0.96" SSD1306 modules.
const OLED_A_SH1106: bool = false;
const OLED_B_SH1106: bool = false;
/// Rotate a panel by 180° if it appears upside down.
const OLED_A_FLIP: bool = false;
const OLED_B_FLIP: bool = false;
const OLED_ADDR: u8 = 0x3C;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("PANIC: {}", info);
    // A real OS restarts after a crash.
    for _ in 0..4_000_000 {
        core::hint::spin_loop();
    }
    esp_hal::system::software_reset()
}

static NET_RES: StaticCell<StackResources<5>> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 150 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    log::info!("Aetheris OS starting");

    // ── displays ────────────────────────────────────────────────────────────────────────
    // OLED A (yellow/blue, main): hardware I2C0 on GPIO6 (SDA) / GPIO7 (SCL)
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);
    let mut oled_a = Oled::new(i2c, OLED_ADDR, OLED_A_SH1106, OLED_A_FLIP);
    // OLED B (blue, secondary): bit-banged I2C on GPIO20 (SDA) / GPIO21 (SCL)
    let soft = SoftI2c::new(peripherals.GPIO20, peripherals.GPIO21, 2);
    let mut oled_b = Oled::new(soft, OLED_ADDR, OLED_B_SH1106, OLED_B_FLIP);
    let a_ok = oled_a.init();
    let b_ok = oled_b.init();
    log::info!("OLED A: {}  OLED B: {}", a_ok, b_ok);

    // ── input / output pins ─────────────────────────────────────────────────────────────
    let mut keypad = Keypad::new(
        [peripherals.GPIO0.degrade(), peripherals.GPIO1.degrade(), peripherals.GPIO2.degrade(), peripherals.GPIO3.degrade()],
        [peripherals.GPIO10.degrade(), peripherals.GPIO11.degrade(), peripherals.GPIO18.degrade(), peripherals.GPIO19.degrade()],
    );
    let mut led = Output::new(peripherals.GPIO22, Level::Low, OutputConfig::default());
    let slave_boot = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let slave_reset = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
    let slave_uart = Uart::new(peripherals.UART1, UartConfig::default().with_baudrate(115_200))
        .unwrap()
        .with_rx(peripherals.GPIO9)
        .with_tx(peripherals.GPIO8)
        .into_async();

    // ── flash + settings ────────────────────────────────────────────────────────────────
    *FLASH.lock().await = Some(esp_storage::FlashStorage::new(peripherals.FLASH));
    storage::mark_valid().await;
    let binfo = storage::boot_info().await;
    let settings = storage::load().await.unwrap_or_default();
    VOLUME.store(settings.volume, Ordering::Relaxed);

    // ── background tasks ────────────────────────────────────────────────────────────────
    spawner.spawn(buzzer_task(peripherals.LEDC, peripherals.GPIO23.degrade()).unwrap());
    spawner.spawn(ir_task(peripherals.GPIO4.degrade()).unwrap());
    spawner.spawn(storage_task().unwrap());

    // Wi-Fi + network stack
    let rng = Rng::new();
    let seed = ((rng.random() as u64) << 32) | rng.random() as u64;
    let ctrl = WifiController::new(peripherals.WIFI, Default::default()).expect("wifi init");
    let sta = Interface::station();
    let mac = sta.mac_address();
    let (stack, runner) = embassy_net::new(sta, embassy_net::Config::dhcpv4(Default::default()), NET_RES.init(StackResources::new()), seed);
    let saved = if settings.wifi_ssid.is_empty() { None } else { Some((settings.wifi_ssid.clone(), settings.wifi_pass.clone())) };
    spawner.spawn(net_runner(runner).unwrap());
    spawner.spawn(wifi_task(ctrl, stack, mac, saved, settings.wifi_auto).unwrap());
    spawner.spawn(http_task(stack, slave_uart, slave_boot, slave_reset).unwrap());

    // ── the OS ──────────────────────────────────────────────────────────────────────────
    let mut sys = Sys::new(settings, seed as u32 ^ (seed >> 32) as u32);
    sys.info = Info { flash_mb: binfo.flash_mb, ota_slot: binfo.slot, storage_ok: binfo.storage_ok, oled_a_ok: a_ok, oled_b_ok: b_ok, ..Default::default() };
    let mut os = Os::new(&sys);
    let mut scanner = KeyScanner::new();
    let mut events: Vec<KeyEvent> = Vec::new();
    let (mut fa, mut fb) = (Fb::new(), Fb::new());

    let mut next_frame = 0u64;
    let mut next_wifi = 0u64;
    let mut led_until = 0u64;
    let mut applied: (u8, u8, bool, bool) = (255, 255, false, false);
    let mut power = Power::On;
    let (mut frames, mut fps_t) = (0u32, 0u64);

    loop {
        let now = Instant::now().as_millis();
        sys.now = now;

        // keypad
        let raw = keypad.scan();
        sys.raw_keys = raw;
        events.clear();
        scanner.update(raw, now, &mut events);
        for ev in events.drain(..) {
            os.key(ev, &mut sys);
        }

        // snapshots from background tasks
        IR.lock(|s| sys.ir = s.borrow().clone());
        if now >= next_wifi {
            next_wifi = now + 200;
            WIFI_STATE.lock(|s| sys.wifi = s.borrow().clone());
        }
        if let Some(unix) = TIME_SYNC.try_take() {
            sys.settings.epoch_off = unix - (now / 1000) as i64;
            sys.save();
        }

        // frame
        if now >= next_frame {
            next_frame = now + 40;
            sys.info.heap_used = esp_alloc::HEAP.used();
            sys.info.heap_free = esp_alloc::HEAP.free();
            os.tick(&mut sys);
            VOLUME.store(sys.settings.volume, Ordering::Relaxed);

            // effects requested by the OS / apps
            let fx: Vec<Effect> = sys.fx.drain(..).collect();
            for e in fx {
                match e {
                    Effect::Snd(s) => {
                        let silent_key = matches!(s, Snd::Click | Snd::Typing) && !sys.settings.key_click;
                        if !silent_key {
                            let _ = SOUND.try_send(SoundCmd::Seq(aetheris::sound::steps(s)));
                        }
                    }
                    Effect::Tone(f) => {
                        let _ = SOUND.try_send(SoundCmd::Tone(f));
                    }
                    Effect::Silence => {
                        let _ = SOUND.try_send(SoundCmd::Silence);
                    }
                    Effect::Play(v) => {
                        let _ = SOUND.try_send(SoundCmd::Owned(v));
                    }
                    Effect::LedPulse(ms) => led_until = now + ms as u64,
                    Effect::Save => SAVE.signal(sys.settings.encode()),
                    Effect::Sleep => os.sleep(),
                    Effect::Wifi(c) => {
                        let _ = WIFI_CMD.try_send(c);
                    }
                    Effect::Reboot => {
                        Timer::after(Duration::from_millis(900)).await;
                        esp_hal::system::software_reset();
                    }
                }
            }

            // status LED: solid when Wi-Fi is connected, blinking while connecting/scanning
            let lvl = if now < led_until {
                true
            } else {
                match sys.wifi.status {
                    WifiStatus::Connected => true,
                    WifiStatus::Connecting => (now / 150) % 2 == 0,
                    WifiStatus::Scanning => (now / 500) % 2 == 0 && (now / 60) % 2 == 0,
                    _ => false,
                }
            };
            led.set_level(if lvl { Level::High } else { Level::Low });

            // power / brightness / invert
            let want = os.power(&sys);
            let cfg = (sys.settings.bright_a, sys.settings.bright_b, sys.settings.invert_a, sys.settings.invert_b);
            if want != power || cfg != applied {
                if want == Power::Off {
                    oled_a.power(false);
                    oled_b.power(false);
                } else {
                    if power == Power::Off {
                        oled_a.power(true);
                        oled_b.power(true);
                    }
                    let (la, lb) = if want == Power::Dim { (0, 0) } else { (cfg.0, cfg.1) };
                    oled_a.set_contrast(la);
                    oled_b.set_contrast(lb);
                    oled_a.set_invert(cfg.2);
                    oled_b.set_invert(cfg.3);
                }
                power = want;
                applied = cfg;
            }

            if power != Power::Off {
                os.draw(&mut fa, &mut fb, &sys);
                oled_a.flush(&fa).await;
                oled_b.flush(&fb).await;
            }

            frames += 1;
            if now - fps_t >= 1000 {
                sys.info.fps = frames;
                frames = 0;
                fps_t = now;
            }
        }
        let _ = Key::A;
        Timer::after(Duration::from_millis(6)).await;
    }
}
