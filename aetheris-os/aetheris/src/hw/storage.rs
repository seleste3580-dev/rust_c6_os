//! Flash storage: persistent settings in the `storage` partition, partition/OTA slot info,
//! and the shared flash handle used by the OTA writer.
use super::shared::SAVE;
use crate::settings::Settings;
use alloc::vec::Vec;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_bootloader_esp_idf::partitions::{DataPartitionSubType, PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table};
use esp_storage::FlashStorage;

pub static FLASH: Mutex<CriticalSectionRawMutex, Option<FlashStorage<'static>>> = Mutex::new(None);

pub struct BootInfo {
    pub slot: &'static str,
    pub flash_mb: u32,
    pub storage_ok: bool,
}

pub async fn boot_info() -> BootInfo {
    let mut g = FLASH.lock().await;
    let mut info = BootInfo { slot: "?", flash_mb: 0, storage_ok: false };
    if let Some(flash) = g.as_mut() {
        info.flash_mb = (esp_storage::FlashStorage::capacity(flash) / (1024 * 1024)) as u32;
        let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
        if let Ok(pt) = read_partition_table(flash, &mut buf) {
            if let Ok(Some(p)) = pt.booted_partition() {
                info.slot = match p.label_as_str() {
                    "ota_0" => "ota_0",
                    "ota_1" => "ota_1",
                    "factory" => "factory",
                    _ => "app",
                };
            }
            info.storage_ok = matches!(pt.find_partition(PartitionType::Data(DataPartitionSubType::Spiffs)), Ok(Some(_)));
        }
    }
    info
}

pub async fn load() -> Option<Settings> {
    let mut g = FLASH.lock().await;
    let flash = g.as_mut()?;
    let mut ptbuf = [0u8; PARTITION_TABLE_MAX_LEN];
    let pt = read_partition_table(flash, &mut ptbuf).ok()?;
    let part = pt.find_partition(PartitionType::Data(DataPartitionSubType::Spiffs)).ok()??;
    let mut region = part.as_flash_region(flash);
    let mut buf = [0u8; crate::settings::MAX_ENCODED];
    region.read(0, &mut buf).ok()?;
    Settings::decode(&buf)
}

async fn write(bytes: &[u8]) -> bool {
    let mut g = FLASH.lock().await;
    let Some(flash) = g.as_mut() else { return false };
    let mut ptbuf = [0u8; PARTITION_TABLE_MAX_LEN];
    let Ok(pt) = read_partition_table(flash, &mut ptbuf) else { return false };
    let Ok(Some(part)) = pt.find_partition(PartitionType::Data(DataPartitionSubType::Spiffs)) else { return false };
    let mut region = part.as_flash_region(flash);
    let mut buf = Vec::from(bytes);
    while buf.len() % 4 != 0 {
        buf.push(0xFF);
    }
    region.erase(0, 4096).is_ok() && region.write(0, &buf).is_ok()
}

/// Debounced writer: waits until settings stop changing for 1.5 s, then programs flash once.
#[embassy_executor::task]
pub async fn storage_task() {
    loop {
        let mut bytes = SAVE.wait().await;
        loop {
            match select(Timer::after(Duration::from_millis(1500)), SAVE.wait()).await {
                Either::First(_) => break,
                Either::Second(b) => bytes = b,
            }
        }
        let ok = write(&bytes).await;
        log::info!("settings saved: {}", ok);
    }
}

/// Tell the bootloader this firmware booted fine (cancels automatic rollback after an OTA update).
pub async fn mark_valid() {
    use esp_bootloader_esp_idf::ota::OtaImageState;
    use esp_bootloader_esp_idf::ota_updater::OtaUpdater;
    let mut g = FLASH.lock().await;
    if let Some(flash) = g.as_mut() {
        let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
        if let Ok(mut up) = OtaUpdater::new(flash, &mut buf) {
            if let Ok(st) = up.current_ota_state() {
                if st == OtaImageState::New || st == OtaImageState::PendingVerify {
                    let _ = up.set_current_ota_state(OtaImageState::Valid);
                    log::info!("OTA image marked valid");
                }
            }
        }
    }
}
