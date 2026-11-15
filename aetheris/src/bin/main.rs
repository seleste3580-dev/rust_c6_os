#![no_std]
#![no_main]

extern crate alloc;

use aetheris::net::{MqttClientService, MqttTelemetryPacket, OtaClient, PwebDbClient};
use esp_hal::{
    clock::ClockControl,
    gpio::Io,
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_hal::entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let _timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);

    // Initialize Status LED (GPIO22)
    let mut status_led = io.pins.gpio22.into_push_pull_output();
    status_led.set_high();

    // Initialize Pweb2 Services
    let _mqtt_svc = MqttClientService::new("aetheris-c6-core", "seltec_esp32_device_token");
    let _ota_client = OtaClient::new();
    let _pweb_db = PwebDbClient::new("seltec_db_device_token");

    // Aetheris OS main dispatch loop
    loop {
        // Sleep until next interrupt or Embassy task schedule
    }
}
