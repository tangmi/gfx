#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;

#[cfg(not(feature = "vulkan"))]
extern crate gfx_backend_empty as back;

#[cfg(not(feature = "vulkan-device-simulation"))]
compile_error!("This test requires the feature, `vulkan-device-simulation`");

use hal::{buffer::Usage, device::Device, prelude::PhysicalDevice, queue::QueueFamily, Instance};
use log::*;

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let device_jsons = std::fs::read_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vulkan-device-simulation/devices"
    ))
    .unwrap()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path());

    for device_json in device_jsons {
        debug!("{:?}", device_json);
        std::env::set_var("VK_DEVSIM_FILENAME", &device_json);

        create_device().unwrap();
    }
}

fn create_device() -> Result<(), Box<dyn std::error::Error>> {
    let instance: back::Instance = hal::Instance::create("vulkan-device-simulation-test", 1)?;
    let adapters = instance.enumerate_adapters();
    for adapter in adapters {
        // Build a new device and associated command queues
        let family = adapter
            .queue_families
            .iter()
            .find(|family| family.queue_type().supports_graphics())
            .expect("No queue family supports graphics");
        let gpu = unsafe {
            adapter
                .physical_device
                .open(&[(family, &[1.0])], hal::Features::empty())?
        };
    }

    Ok(())
}
