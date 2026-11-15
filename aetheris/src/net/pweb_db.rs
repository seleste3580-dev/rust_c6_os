use alloc::format;
use alloc::string::String;

pub struct PwebDbClient {
    pub base_url: &'static str,
    pub api_key: String,
}

impl PwebDbClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            base_url: "https://seltec.dev",
            api_key: String::from(api_key),
        }
    }

    /// Read dynamic rows from any allowed table (entries, media, iot_telemetry, etc.)
    pub fn build_read_url(&self, table: &str, limit: u32) -> String {
        format!("{}/api/v1/db/{}?limit={}", self.base_url, table, limit)
    }

    /// Write structured records into Pweb2 database
    pub fn build_write_url(&self, table: &str) -> String {
        format!("{}/api/v1/db/{}", self.base_url, table)
    }

    /// Ingest telemetry directly to the Pweb2 IoT fleet telemetry engine
    pub fn build_telemetry_url(&self) -> String {
        format!("{}/api/v1/iot/telemetry", self.base_url)
    }

    /// Poll pending commands dispatched from the Pweb2 Admin Control Plane
    pub fn build_command_poll_url(&self, device_id: &str) -> String {
        format!("{}/api/v1/iot/command/{}", self.base_url, device_id)
    }

    /// Acknowledge executed hardware commands back to Pweb2 Admin Panel
    pub fn build_command_ack_url(&self, device_id: &str) -> String {
        format!("{}/api/v1/iot/command/{}/ack", self.base_url, device_id)
    }

    /// Inspect device state registered in Pweb2
    pub fn build_status_url(&self, device_id: &str) -> String {
        format!("{}/api/v1/iot/status/{}", self.base_url, device_id)
    }

    /// API authentication headers
    pub fn auth_header(&self) -> (&'static str, &str) {
        ("X-API-Key", &self.api_key)
    }

    /// Format command acknowledgment JSON payload
    pub fn format_command_ack_payload(&self, command_id: &str, status: &str, result: &str) -> String {
        format!(
            "{{\"command_id\":\"{}\",\"status\":\"{}\",\"result\":\"{}\"}}",
            command_id, status, result
        )
    }

    /// Format structured IoT telemetry payload for Pweb2 ingestion
    pub fn format_telemetry_payload(
        &self,
        device_id: &str,
        hw: &str,
        fw: &str,
        battery_mv: u16,
        rssi: i8,
        custom_json: &str,
    ) -> String {
        format!(
            "{{\"device_id\":\"{}\",\"hardware\":\"{}\",\"firmware_version\":\"{}\",\"battery_mv\":{},\"rssi\":{},\"data\":{}}}",
            device_id, hw, fw, battery_mv, rssi, custom_json
        )
    }
}
