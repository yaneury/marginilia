// Color convention for the Waveshare 7.5" V2 (UC8179) is inverted at the hardware level:
// Color::White (1-bits) renders as physical black; Color::Black (0-bits) renders as physical white.
// All color values below are intentionally swapped to compensate.
use core::fmt::Write;

use embedded_graphics::{
    geometry::Size,
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_8X13_ITALIC, FONT_9X18_BOLD, FONT_10X20},
    },
    prelude::*,
    primitives::Rectangle,
};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};
use embedded_text::{
    TextBox,
    alignment::{HorizontalAlignment, VerticalAlignment},
    style::TextBoxStyleBuilder,
};
use epd_waveshare::{
    color::Color,
    epd7in5_v2::{Display7in5, Epd7in5},
    prelude::*,
};

use crate::error::Error;
use crate::model::Quote;

const MARGIN: i32 = 40;
const CONTENT_WIDTH: u32 = 720;
const BODY_HEIGHT: u32 = 340;
const AUTHOR_Y: i32 = 390;
const AUTHOR_HEIGHT: u32 = 22;
const WORK_Y: i32 = 416;
const WORK_HEIGHT: u32 = 16;
const FULL_HEIGHT: u32 = 400;

pub struct Display<SPI, BUSY, DC, RST, DELAY> {
    epd: Epd7in5<SPI, BUSY, DC, RST, DELAY>,
    spi: SPI,
    delay: DELAY,
    buffer: Display7in5,
}

impl<SPI, BUSY, DC, RST, DELAY> Display<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    pub fn new(
        mut spi: SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        mut delay: DELAY,
    ) -> Result<Self, Error<SPI::Error>> {
        let epd = Epd7in5::new(&mut spi, busy, dc, rst, &mut delay, None).map_err(Error::Spi)?;
        Ok(Self {
            epd,
            spi,
            delay,
            buffer: Display7in5::default(),
        })
    }

    pub fn show_quote(&mut self, quote: &Quote) -> Result<(), Error<SPI::Error>> {
        self.buffer.clear(Color::Black)?;

        let text_style = MonoTextStyle::new(&FONT_10X20, Color::White);
        let centered = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Middle)
            .build();

        TextBox::with_textbox_style(
            quote.body,
            Rectangle::new(
                Point::new(MARGIN, MARGIN),
                Size::new(CONTENT_WIDTH, BODY_HEIGHT),
            ),
            text_style,
            centered,
        )
        .draw(&mut self.buffer)?;

        let right_aligned = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Middle)
            .build();

        TextBox::with_textbox_style(
            quote.author,
            Rectangle::new(Point::new(MARGIN, AUTHOR_Y), Size::new(CONTENT_WIDTH, AUTHOR_HEIGHT)),
            MonoTextStyle::new(&FONT_9X18_BOLD, Color::White),
            right_aligned,
        )
        .draw(&mut self.buffer)?;

        TextBox::with_textbox_style(
            quote.work,
            Rectangle::new(Point::new(MARGIN, WORK_Y), Size::new(CONTENT_WIDTH, WORK_HEIGHT)),
            MonoTextStyle::new(&FONT_8X13_ITALIC, Color::White),
            right_aligned,
        )
        .draw(&mut self.buffer)?;

        self.flush()
    }

    pub fn show_standby(&mut self) -> Result<(), Error<SPI::Error>> {
        self.buffer.clear(Color::Black)?;

        let style = MonoTextStyle::new(&FONT_10X20, Color::White);
        let centered = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Middle)
            .build();

        TextBox::with_textbox_style(
            "Updating quote bank...",
            Rectangle::new(
                Point::new(MARGIN, MARGIN),
                Size::new(CONTENT_WIDTH, FULL_HEIGHT),
            ),
            style,
            centered,
        )
        .draw(&mut self.buffer)?;

        self.flush()
    }

    pub fn show_error<E: core::fmt::Display>(
        &mut self,
        error: &E,
    ) -> Result<(), Error<SPI::Error>> {
        self.buffer.clear(Color::Black)?;

        let mut message = heapless::String::<512>::new();
        write!(message, "{error}").ok();

        let style = MonoTextStyle::new(&FONT_10X20, Color::White);
        let centered = TextBoxStyleBuilder::new()
            .alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Middle)
            .build();

        TextBox::with_textbox_style(
            &message,
            Rectangle::new(
                Point::new(MARGIN, MARGIN),
                Size::new(CONTENT_WIDTH, FULL_HEIGHT),
            ),
            style,
            centered,
        )
        .draw(&mut self.buffer)?;

        self.flush()
    }

    fn flush(&mut self) -> Result<(), Error<SPI::Error>> {
        self.epd
            .wake_up(&mut self.spi, &mut self.delay)
            .map_err(Error::Spi)?;
        self.epd
            .update_frame(&mut self.spi, self.buffer.buffer(), &mut self.delay)
            .map_err(Error::Spi)?;
        self.epd
            .display_frame(&mut self.spi, &mut self.delay)
            .map_err(Error::Spi)?;
        self.epd
            .sleep(&mut self.spi, &mut self.delay)
            .map_err(Error::Spi)
    }
}
