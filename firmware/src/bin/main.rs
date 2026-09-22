#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_hal::delay::DelayNs;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    rng::Rng,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};
use esp_println::println;
use marginilia::{display::Display, sample::QUOTES};

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

    println!("Marginilia: starting...");

    let dc = Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default());
    let cs = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
    let busy = Input::new(
        peripherals.GPIO25,
        InputConfig::default().with_pull(Pull::None),
    );

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO14);

    let spi_dev = ExclusiveDevice::new(spi, cs, Delay::new()).unwrap();
    let mut display = Display::new(spi_dev, busy, dc, rst, Delay::new()).unwrap();
    let mut delay = Delay::new();
    let rng = Rng::new();

    const N: usize = QUOTES.len();
    let mut deck: [u8; N] = core::array::from_fn(|i| i as u8);
    let mut pos = N; // start at end so first iteration triggers a shuffle

    loop {
        if pos >= N {
            // Fisher-Yates shuffle
            for i in (1..N).rev() {
                let j = (rng.random() as usize) % (i + 1);
                deck.swap(i, j);
            }
            pos = 0;
        }
        let idx = deck[pos] as usize;
        pos += 1;
        println!("Marginilia: quote {}/{}", pos, N);
        display.show_quote(&QUOTES[idx]).unwrap();
        delay.delay_ms(3 * 60 * 1_000u32);
    }
}
