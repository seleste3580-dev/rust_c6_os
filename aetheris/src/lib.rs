#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod hw;
pub mod net;
pub mod apps;
pub mod os;
pub mod sys;
pub mod settings;
pub mod input;
pub mod fb;
pub mod gfx;
pub mod icons;
pub mod sound;

pub const OS_NAME: &str = "Aetheris OS";
pub const OS_VERSION: &str = "v1.0.0";
pub const OS_HARDWARE_TARGET: &str = "ESP32-C6";
pub const OS_VENDOR: &str = "Seleste Technologies Ltd";

// Pweb2 Integration Parameters
pub const PWEB2_DEFAULT_HOST: &str = "https://seltec.dev";
pub const PWEB2_MQTT_HOST: &str = "mqtt.seltec.dev";
pub const PWEB2_MQTT_PORT: u16 = 8883;
pub const PWEB2_MQTT_TOPIC_TELEMETRY: &str = "devices/aetheris-c6-core/telemetry";
pub const PWEB2_MQTT_TOPIC_COMMAND: &str = "devices/aetheris-c6-core/command";
pub const PWEB2_MQTT_TOPIC_STATUS: &str = "devices/aetheris-c6-core/status";
pub const PWEB2_OTA_CHECK_ENDPOINT: &str = "/api/v1/ota/check?hw=ESP32-C6";
