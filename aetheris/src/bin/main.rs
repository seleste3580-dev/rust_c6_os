#![no_std]
#![no_main]

extern crate alloc;

use core::cell::UnsafeCell;
use aetheris::net::{MqttClientService, OtaClient, PwebDbClient};

// Interior mutability allocator for Rust 2024 compliance (no static mut references)
struct SimpleAllocator {
    heap: UnsafeCell<[u8; 64 * 1024]>,
    offset: UnsafeCell<usize>,
}

unsafe impl Sync for SimpleAllocator {}

unsafe impl core::alloc::GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe {
            let offset_ptr = self.offset.get();
            let current = *offset_ptr;
            let align = layout.align();
            let size = layout.size();
            let start = (current + align - 1) & !(align - 1);
            if start + size > 64 * 1024 {
                core::ptr::null_mut()
            } else {
                *offset_ptr = start + size;
                let heap_ptr = self.heap.get() as *mut u8;
                heap_ptr.add(start)
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator {
    heap: UnsafeCell::new([0; 64 * 1024]),
    offset: UnsafeCell::new(0),
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialize Status LED (GPIO22)
    let mut status_led = esp_hal::gpio::Output::new(
        peripherals.GPIO22,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );
    status_led.set_high();

    // Initialize Pweb2 Services
    let _mqtt_svc = MqttClientService::new("aetheris-c6-core", "seltec_esp32_device_token");
    let _ota_client = OtaClient::new();
    let _pweb_db = PwebDbClient::new("seltec_db_device_token");

    // Aetheris OS main dispatch loop
    loop {
        // Yield to Embassy async scheduler or sleep
    }
}
