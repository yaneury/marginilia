#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{
    epd7in5_v2::Epd7in5,
    prelude::*,
};
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    spi::{
        master::{Config, Spi},
        Mode,
    },
    time::Rate,
};
use esp_println::println;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("Clearing display...");

    let mut delay = Delay::new();

    let dc   = Output::new(peripherals.GPIO27, Level::Low,  OutputConfig::default());
    let rst  = Output::new(peripherals.GPIO26, Level::Low,  OutputConfig::default());
    let cs   = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
    let busy = Input::new(peripherals.GPIO25,  InputConfig::default().with_pull(Pull::None));

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO14);

    let mut spi_dev = ExclusiveDevice::new(spi, cs, Delay::new()).unwrap();
    let mut epd = Epd7in5::new(&mut spi_dev, busy, dc, rst, &mut delay, None).unwrap();

    epd.clear_frame(&mut spi_dev, &mut delay).unwrap();
    epd.display_frame(&mut spi_dev, &mut delay).unwrap();

    println!("Done.");

    epd.sleep(&mut spi_dev, &mut delay).unwrap();

    loop {}
}
