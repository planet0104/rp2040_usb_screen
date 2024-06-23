use core::iter;

use anyhow::Result;
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_futures::block_on;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{PIN_13, PIN_14, PIN_9, SPI0};
use embassy_rp::spi::{Blocking, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use embedded_hal_1::digital::OutputPin;
use embedded_hal_1::spi::SpiDevice;
use super::ST7789;

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

pub fn draw_rgb565_le(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<NoopRawMutex, Spi<SPI0, Blocking>, Output<PIN_9>>, Output<PIN_13>>, Output<PIN_14>>, image: &[u8], x:u16, y:u16, width: u16, height: u16){
    let colors = image.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]]) );
    let _ = display.set_pixels(x, y, width, height, colors);
}

pub fn draw_rgb565_u8(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<NoopRawMutex, Spi<SPI0, Blocking>, Output<PIN_9>>, Output<PIN_13>>, Output<PIN_14>>, image: &[u8], x:u16, y:u16, width: u16, height: u16){
    let _ = display.set_pixels_u8(x, y, width, height, image);
}

pub fn clear_rect(display: &mut ST7789<SPIDeviceInterface<SpiDeviceWithConfig<NoopRawMutex, Spi<SPI0, Blocking>, Output<PIN_9>>, Output<PIN_13>>, Output<PIN_14>>, color: u16, x:u16, y:u16, width: u16, height: u16){
    let colors = iter::repeat(color).take(width as usize *height as usize);
    let _ = display.set_pixels(x, y, width, height, colors);
}