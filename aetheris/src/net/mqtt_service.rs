use alloc::format;
use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttTelemetryPacket {
    pub device_id: String,
    pub hw: String,
    pub os_version: String,
    pub uptime_s: u64,
    pub free_heap_kb: u32,
    pub battery_mv: u16,
    pub wifi_rssi: i8,
    pub current_app: String,
    pub oled_a_on: bool,
    pub oled_b_on: bool,
}

impl MqttTelemetryPacket {
    pub fn new(device_id: &str, uptime_s: u64, free_heap_kb: u32, battery_mv: u16, rssi: i8, app: &str) -> Self {
        Self {
            device_id: String::from(device_id),
            hw: String::from("ESP32-C6"),
            os_version: String::from("v1.0.0"),
            uptime_s,
            free_heap_kb,
            battery_mv,
            wifi_rssi: rssi,
            current_app: String::from(app),
            oled_a_on: true,
            oled_b_on: true,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"device_id\":\"{}\",\"hw\":\"{}\",\"version\":\"{}\",\"uptime_s\":{},\"heap_kb\":{},\"batt_mv\":{},\"rssi\":{},\"app\":\"{}\"}}",
            self.device_id, self.hw, self.os_version, self.uptime_s, self.free_heap_kb, self.battery_mv, self.wifi_rssi, self.current_app
        )
    }
}

pub struct MqttClientService {
    pub broker_host: &'static str,
    pub broker_port: u16,
    pub device_id: String,
    pub token: String,
    pub is_connected: bool,
    pub last_packet_ack: u64,
}

impl MqttClientService {
    pub fn new(device_id: &str, token: &str) -> Self {
        Self {
            broker_host: "mqtt.seltec.dev",
            broker_port: 8883,
            device_id: String::from(device_id),
            token: String::from(token),
            is_connected: false,
            last_packet_ack: 0,
        }
    }

    pub fn topic_telemetry(&self) -> String {
        format!("devices/{}/telemetry", self.device_id)
    }

    pub fn topic_command(&self) -> String {
        format!("devices/{}/command", self.device_id)
    }

    pub fn topic_status(&self) -> String {
        format!("devices/{}/status", self.device_id)
    }

    pub fn format_lwt_payload(&self) -> String {
        format!("{{\"device_id\":\"{}\",\"status\":\"offline\"}}", self.device_id)
    }

    pub fn format_online_payload(&self) -> String {
        format!("{{\"device_id\":\"{}\",\"status\":\"online\",\"os\":\"Aetheris OS v1.0.0\"}}", self.device_id)
    }

    pub fn parse_command(&self, payload: &str) -> Option<String> {
        if payload.contains("\"reboot\"") {
            Some(String::from("reboot"))
        } else if payload.contains("\"lock\"") {
            Some(String::from("lock"))
        } else if payload.contains("\"beep\"") {
            Some(String::from("beep"))
        } else if payload.contains("\"ota_check\"") {
            Some(String::from("ota_check"))
        } else {
            None
        }
    }
}
