pub mod settings;

pub use settings::SettingsApp;

pub trait App {
    fn name(&self) -> &'static str;
    fn icon(&self) -> u8;
    fn tick(&mut self);
    fn on_key(&mut self, key: char) -> bool;
}
