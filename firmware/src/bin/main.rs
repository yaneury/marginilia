#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

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
use esp_storage::FlashStorage;
use marginilia::{display::Display, sample::QUOTES, storage::QuoteStore};

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

    let flash = FlashStorage::new();
    let mut store = QuoteStore::new(flash);

    if !store.is_initialized() {
        println!("Marginilia: formatting flash store...");
        store.format().expect("format failed");
    }

    let stored_count = store.count().unwrap_or(0);
    println!("Marginilia: {} quotes in flash", stored_count);

    if stored_count > 0 {
        run_flash_carousel(&mut display, &mut delay, &mut store, &rng, stored_count);
    } else {
        println!("Marginilia: flash empty, using sample quotes");
        run_sample_carousel(&mut display, &mut delay, &rng);
    }
}

fn run_flash_carousel(
    display: &mut Display<
        impl embedded_hal::spi::SpiDevice,
        impl embedded_hal::digital::InputPin,
        impl embedded_hal::digital::OutputPin,
        impl embedded_hal::digital::OutputPin,
        impl embedded_hal::delay::DelayNs,
    >,
    delay: &mut impl embedded_hal::delay::DelayNs,
    store: &mut QuoteStore<FlashStorage>,
    rng: &Rng,
    count: u32,
) -> ! {
    const MAX_QUOTES: usize = 510;
    const BUF_LEN: usize = 4096;

    let n = (count as usize).min(MAX_QUOTES);
    let mut deck = [0u16; MAX_QUOTES];
    for (i, slot) in deck[..n].iter_mut().enumerate() {
        *slot = i as u16;
    }
    let mut pos = n;
    let mut buf = [0u8; BUF_LEN];

    loop {
        if pos >= n {
            for i in (1..n).rev() {
                let j = (rng.random() as usize) % (i + 1);
                deck.swap(i, j);
            }
            pos = 0;
        }

        let idx = deck[pos] as u32;
        pos += 1;
        println!("Marginilia: flash quote {}/{}", pos, n);

        match store.fetch(idx, &mut buf) {
            Ok(quote) => display.show_quote(&quote).unwrap_or_else(|_| {
                println!("Marginilia: display error");
            }),
            Err(_) => println!("Marginilia: fetch error for index {}", idx),
        }

        delay.delay_ms(3 * 60 * 1_000u32);
    }
}

fn run_sample_carousel(
    display: &mut Display<
        impl embedded_hal::spi::SpiDevice,
        impl embedded_hal::digital::InputPin,
        impl embedded_hal::digital::OutputPin,
        impl embedded_hal::digital::OutputPin,
        impl embedded_hal::delay::DelayNs,
    >,
    delay: &mut impl embedded_hal::delay::DelayNs,
    rng: &Rng,
) -> ! {
    const N: usize = QUOTES.len();
    let mut deck: [u8; N] = core::array::from_fn(|i| i as u8);
    let mut pos = N;

    loop {
        if pos >= N {
            for i in (1..N).rev() {
                let j = (rng.random() as usize) % (i + 1);
                deck.swap(i, j);
            }
            pos = 0;
        }
        let idx = deck[pos] as usize;
        pos += 1;
        println!("Marginilia: sample quote {}/{}", pos, N);
        display.show_quote(&QUOTES[idx]).unwrap();
        delay.delay_ms(3 * 60 * 1_000u32);
    }
}
