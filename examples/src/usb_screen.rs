use futures_lite::future::block_on;
use image::{Rgb, RgbImage};
use nusb::Interface;
use anyhow::Result;
use serialport::{SerialPort, SerialPortInfo, SerialPortType};

use crate::rgb565::rgb888_to_rgb565_be;

pub const BULK_OUT_EP: u8 = 0x01;
pub const BULK_IN_EP: u8 = 0x81;

pub fn open_usb_screen() -> Result<Option<Interface>>{
    let mut di = nusb::list_devices()?;
    for d in di{
        if d.serial_number().unwrap_or("").starts_with("USBSCR"){
            let device = d.open()?;
            let interface = device.claim_interface(0)?;
            return Ok(Some(interface));
        }
    }
    Ok(None)
}

pub fn find_usb_serial_device() -> Result<Vec<SerialPortInfo>>{
    let ports: Vec<SerialPortInfo> = serialport::available_ports().unwrap_or(vec![]);
    let mut usb_screen = vec![];
    for p in ports {
        match p.port_type.clone(){
            SerialPortType::UsbPort(port) => {
                if port.serial_number.unwrap_or("".to_string()).starts_with("USBSCR"){
                    usb_screen.push(p);
                    continue;
                }
            }
            _ => ()
        }
    }
    Ok(usb_screen)
}

pub fn clear_screen(color: Rgb<u8>, interface:&Interface, width: u16, height: u16) -> anyhow::Result<()>{
    let mut img = RgbImage::new(width as u32, height as u32);
    for p in img.pixels_mut(){
        *p = color;
    }
    draw_rgb_image(0, 0, &img, interface)
}

pub fn clear_screen_serial(color: Rgb<u8>, port:&mut dyn SerialPort, width: u16, height: u16) -> anyhow::Result<()>{
    let mut img = RgbImage::new(width as u32, height as u32);
    for p in img.pixels_mut(){
        *p = color;
    }
    draw_rgb_image_serial(0, 0, &img, port)
}

pub fn draw_rgb_image(x: u16, y: u16, img:&RgbImage, interface:&Interface) -> anyhow::Result<()>{
    //ST7789驱动使用的是Big-Endian
    let rgb565 = rgb888_to_rgb565_be(&img, img.width() as usize, img.height() as usize);
    draw_rgb565(&rgb565, x, y, img.width() as u16, img.height() as u16, interface)
}

pub fn draw_rgb565(rgb565:&[u8], x: u16, y: u16, width: u16, height: u16, interface:&Interface) -> anyhow::Result<()>{
    let rgb565_u8_slice = lz4_flex::compress_prepend_size(rgb565);

    const IMAGE_AA:u64 = 7596835243154170209;
    const BOOT_USB:u64 = 7093010483740242786;
    const IMAGE_BB:u64 = 7596835243154170466;

    let img_begin = &mut [0u8; 16];
    img_begin[0..8].copy_from_slice(&IMAGE_AA.to_be_bytes());
    img_begin[8..10].copy_from_slice(&width.to_be_bytes());
    img_begin[10..12].copy_from_slice(&height.to_be_bytes());
    img_begin[12..14].copy_from_slice(&x.to_be_bytes());
    img_begin[14..16].copy_from_slice(&y.to_be_bytes());
    // println!("draw:{x}x{y} {width}x{height}");

    block_on(interface.bulk_out(BULK_OUT_EP, img_begin.into())).status?;
    //读取
    // let result = block_on(interface.bulk_in(BULK_IN_EP, RequestBuffer::new(64))).data;
    // let msg = String::from_utf8(result)?;
    // println!("{msg}ms");

    block_on(interface.bulk_out(BULK_OUT_EP, rgb565_u8_slice.into())).status?;
    block_on(interface.bulk_out(BULK_OUT_EP, IMAGE_BB.to_be_bytes().into())).status?;
    Ok(())
}

pub fn draw_rgb_image_serial(x: u16, y: u16, img:&RgbImage, port:&mut dyn SerialPort) -> anyhow::Result<()>{
    //ST7789驱动使用的是Big-Endian
    let rgb565 = rgb888_to_rgb565_be(&img, img.width() as usize, img.height() as usize);
    draw_rgb565_serial(&rgb565, x, y, img.width() as u16, img.height() as u16, port)
}

pub fn draw_rgb565_serial(rgb565:&[u8], x: u16, y: u16, width: u16, height: u16, port:&mut dyn SerialPort) -> anyhow::Result<()>{
    let rgb565_u8_slice = lz4_flex::compress_prepend_size(rgb565);

    const IMAGE_AA:u64 = 7596835243154170209;
    const BOOT_USB:u64 = 7093010483740242786;
    const IMAGE_BB:u64 = 7596835243154170466;

    let img_begin = &mut [0u8; 16];
    img_begin[0..8].copy_from_slice(&IMAGE_AA.to_be_bytes());
    img_begin[8..10].copy_from_slice(&width.to_be_bytes());
    img_begin[10..12].copy_from_slice(&height.to_be_bytes());
    img_begin[12..14].copy_from_slice(&x.to_be_bytes());
    img_begin[14..16].copy_from_slice(&y.to_be_bytes());
    // println!("draw:{x}x{y} {width}x{height} len={}", rgb565_u8_slice.len());

    port.write(img_begin)?;
    port.flush()?;
    port.write(&rgb565_u8_slice)?;
    port.flush()?;
    port.write(&IMAGE_BB.to_be_bytes())?;
    port.flush()?;
    Ok(())
}