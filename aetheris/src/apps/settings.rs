use super::App;
use crate::net::ota_client::OtaCheckResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsMenu {
    Main,
    WiFi,
    Display,
    Sound,
    Security,
    DateTime,
    IRSensor,
    MqttCloud,
    CheckUpdates,
    System,
    About,
}

pub struct SettingsApp {
    pub current_menu: SettingsMenu,
    pub selected_index: usize,
    pub ota_status: Option<OtaCheckResponse>,
    pub checking_ota: bool,
    pub mqtt_connected: bool,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self {
            current_menu: SettingsMenu::Main,
            selected_index: 0,
            ota_status: None,
            checking_ota: false,
            mqtt_connected: true,
        }
    }

    pub fn menu_items(&self) -> &'static [&'static str] {
        match self.current_menu {
            SettingsMenu::Main => &[
                "1. Wi-Fi Network",
                "2. Display & Wallpapers",
                "3. Sound & Volume",
                "4. Security & Pin",
                "5. Date & Time",
                "6. IR Sensor",
                "7. MQTT Cloud Link",
                "8. Check for Updates",
                "9. System Information",
                "10. About Aetheris OS",
            ],
            SettingsMenu::MqttCloud => &[
                "Broker: mqtt.seltec.dev",
                "TLS Port: 8883 (Active)",
                "Status: Connected",
                "Topic: devices/c6/telemetry",
                "Sync DB: Active",
                "<- Back to Menu",
            ],
            SettingsMenu::CheckUpdates => &[
                "Server: seltec.dev",
                "Current: v1.0.0",
                "Check Updates Now [#]",
                "Auto-OTA: Enabled",
                "<- Back to Menu",
            ],
            _ => &["<- Back to Menu"],
        }
    }
}

impl App for SettingsApp {
    fn name(&self) -> &'static str {
        "Settings"
    }

    fn icon(&self) -> u8 {
        1
    }

    fn tick(&mut self) {}

    fn on_key(&mut self, key: char) -> bool {
        match key {
            '2' => { // Up
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            '8' => { // Down
                let max = self.menu_items().len().saturating_sub(1);
                if self.selected_index < max {
                    self.selected_index += 1;
                }
                true
            }
            '5' | '#' => { // Select
                match self.current_menu {
                    SettingsMenu::Main => {
                        match self.selected_index {
                            6 => self.current_menu = SettingsMenu::MqttCloud,
                            7 => self.current_menu = SettingsMenu::CheckUpdates,
                            _ => {}
                        }
                        self.selected_index = 0;
                    }
                    SettingsMenu::CheckUpdates => {
                        if self.selected_index == 2 {
                            self.checking_ota = true;
                        } else if self.selected_index == 4 {
                            self.current_menu = SettingsMenu::Main;
                            self.selected_index = 0;
                        }
                    }
                    SettingsMenu::MqttCloud => {
                        if self.selected_index == 5 {
                            self.current_menu = SettingsMenu::Main;
                            self.selected_index = 0;
                        }
                    }
                    _ => {}
                }
                true
            }
            '*' => { // Back
                if self.current_menu != SettingsMenu::Main {
                    self.current_menu = SettingsMenu::Main;
                    self.selected_index = 0;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
