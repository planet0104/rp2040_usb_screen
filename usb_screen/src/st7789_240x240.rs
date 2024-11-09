//https://github.com/embassy-rs/embassy/blob/1cfd5370ac012814b7b386ba9ad8499529bdde4e/examples/rp/src/bin/spi_display.rs#L203

use core::cell::RefCell;
use core::iter;

use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embassy_rp::peripherals::{PIN_13, PIN_14, PIN_15, PIN_6, PIN_7, PIN_9, SPI0};
use embedded_hal_1::digital::OutputPin;
use embedded_hal_1::spi::SpiDevice;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi;
use embassy_rp::spi::{Blocking, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::{Delay, Timer};
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use fixed::traits::FixedOptionalFeatures;
use st7789::{Orientation, ST7789};
// use crate::usb_serial;

const DISPLAY_FREQ: u32 = 64_000_000;

/// SPI display interface.
///
/// This combines the SPI peripheral and a data/command pin
pub struct SPIDeviceInterface<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<SPI, DC> SPIDeviceInterface<SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    /// Create new SPI interface for communciation with a display driver
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }
}

impl<SPI, DC> WriteOnlyDataCommand for SPIDeviceInterface<SPI, DC>
where
    SPI: SpiDevice,
    DC: OutputPin,
{
    fn send_commands(&mut self, cmds: DataFormat<'_>) -> Result<(), DisplayError> {
        // 1 = data, 0 = command
        self.dc.set_low().map_err(|_| DisplayError::DCError)?;

        send_u8(&mut self.spi, cmds).map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        // 1 = data, 0 = command
        self.dc.set_high().map_err(|_| DisplayError::DCError)?;

        send_u8(&mut self.spi, buf).map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }
}

fn send_u8<T: SpiDevice>(spi: &mut T, words: DataFormat<'_>) -> Result<(), T::Error> {
    match words {
        DataFormat::U8(slice) => spi.write(slice),
        DataFormat::U16(slice) => {
            use byte_slice_cast::*;
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16LE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_le();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16BE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_be();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U8Iter(iter) => {
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.into_iter() {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf)?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i])?;
            }

            Ok(())
        }
        DataFormat::U16LEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.map(u16::to_le) {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        DataFormat::U16BEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 64];
            let mut i = 0;
            let len = buf.len();

            for v in iter.map(u16::to_be) {
                buf[i] = v;
                i += 1;

                if i == len {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        _ => unimplemented!(),
    }
}

#[embassy_executor::task]
pub async fn display_task(spi: SPI0, p6: PIN_6, p7: PIN_7, p9: PIN_9, p13: PIN_13, p14: PIN_14, p15: PIN_15){
    let _ = run_display_task(spi, p6, p7, p9, p13, p14, p15).await;
}

pub async fn run_display_task(spi: SPI0, p6: PIN_6, p7: PIN_7, p9: PIN_9, p13: PIN_13, p14: PIN_14, p15: PIN_15) -> anyhow::Result<()> {
    /*
    GND
    VCC
    SCL > clk (PIN6)
    SDA > mosi (PIN7)
    RESET > rst (PIN14)
    DC > dc(PIN13)
    CS > cs(PIN9)
    BL > bl(VCC)
    */
   
    let bl = p15;
    let rst = p14;
    let display_cs = p9;
    let dcx = p13;
    let mosi = p7;
    let clk = p6;

    // create SPI
    let mut display_config = spi::Config::default();
    display_config.frequency = DISPLAY_FREQ;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let spi: Spi<'_, _, Blocking> = Spi::new_blocking_txonly(spi, clk, mosi, display_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let display_spi = SpiDeviceWithConfig::new(&spi_bus, Output::new(display_cs, Level::High), display_config);

    let dcx = Output::new(dcx, Level::Low);
    let rst = Output::new(rst, Level::Low);
    // dcx: 0 = command, 1 = data

    // Enable LCD backlight
    let bl = Output::new(bl, Level::High);

    // display interface abstraction from SPI and DC
    let di = SPIDeviceInterface::new(display_spi, dcx);

    // create driver
    let mut display = ST7789::new(di, Some(rst), Some(bl), 240, 240);

    // initialize
    display.init(&mut Delay).unwrap();

    // set default orientation
    display.set_orientation(Orientation::Landscape).unwrap();

    let mut x = 0;
    loop {
        if x%2 == 0{
            let colors = core::iter::repeat(RawU16::from(Rgb565::RED).into_inner()).take(240 * 240);
            display.set_pixels(0, 0, 239, 239, colors).unwrap();
        }else{
            let colors = core::iter::repeat(RawU16::from(Rgb565::BLUE).into_inner()).take(240 * 240);
            display.set_pixels(0, 0, 239, 239, colors).unwrap();
        }
        Timer::after_secs(1).await;
        x += 1;
    }
}

pub fn draw_rgb565_le(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, SPI0, Blocking>, Output<'_, PIN_9>>, Output<'_, PIN_13>>, Output<'_, PIN_14>, Output<'_, PIN_15>>, image: &[u8], x:u16, y:u16, width: u16, height: u16){
    let colors = image.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]]) );
    let _ = display.set_pixels(x, y, x+width-1, y+height-1, colors);
}

pub fn draw_rgb565_u8(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, SPI0, Blocking>, Output<'_, PIN_9>>, Output<'_, PIN_13>>, Output<'_, PIN_14>, Output<'_, PIN_15>>, image: &[u8], x:u16, y:u16, width: u16, height: u16){
    let colors = image.chunks(2).map(|p| u16::from_be_bytes([p[0], p[1]]) );
    let _ = display.set_pixels(x, y, x+width-1, y+height-1, colors);
}

pub fn clear_rect(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<'_, NoopRawMutex, Spi<'_, SPI0, Blocking>, Output<'_, PIN_9>>, Output<'_, PIN_13>>, Output<'_, PIN_14>, Output<'_, PIN_15>>, color: u16, x:u16, y:u16, width: u16, height: u16){
    let colors = iter::repeat(color).take(width as usize *height as usize);
    let _ = display.set_pixels(x, y, x+width-1, y+height-1, colors);
}