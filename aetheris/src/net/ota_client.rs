use alloc::format;
use alloc::string::String;

#[derive(Debug, Clone, Default)]
pub struct OtaCheckResponse {
    pub update_available: bool,
    pub latest_version: String,
    pub download_url: String,
    pub sha256: String,
    pub release_notes: String,
}

pub struct OtaClient {
    pub pweb2_url: &'static str,
    pub current_version: &'static str,
    pub target_hardware: &'static str,
}

impl OtaClient {
    pub fn new() -> Self {
        Self {
            pweb2_url: "https://seltec.dev",
            current_version: "v1.0.0",
            target_hardware: "ESP32-C6",
        }
    }

    pub fn build_check_url(&self) -> String {
        format!(
            "{}/api/v1/ota/check?hw={}&v={}",
            self.pweb2_url, self.target_hardware, self.current_version
        )
    }

    pub fn parse_check_response(&self, json_body: &str) -> OtaCheckResponse {
        let update_avail = json_body.contains("\"update_available\":true")
            || json_body.contains("\"update_available\": true");

        let extract_str = |key: &str| -> String {
            if let Some(idx) = json_body.find(key) {
                let rest = &json_body[idx + key.len()..];
                if let Some(quote_start) = rest.find('"') {
                    let after_quote = &rest[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        return String::from(&after_quote[..quote_end]);
                    }
                }
            }
            String::new()
        };

        OtaCheckResponse {
            update_available: update_avail,
            latest_version: extract_str("\"latest_version\""),
            download_url: extract_str("\"download_url\""),
            sha256: extract_str("\"sha256\""),
            release_notes: extract_str("\"release_notes\""),
        }
    }

    pub fn resolve_download_url(&self, raw_url: &str) -> String {
        if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            String::from(raw_url)
        } else if raw_url.starts_with('/') {
            format!("{}{}", self.pweb2_url, raw_url)
        } else {
            format!("{}/media/{}", self.pweb2_url, raw_url)
        }
    }
}
