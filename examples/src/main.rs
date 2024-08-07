use std::{thread::sleep, time::{Duration, Instant}};
use anyhow::Result;
use image::open;
use rgb565::rgb888_to_rgb565_le;
use usb_screen::find_usb_serial_device;
mod rgb565;
mod rgb2yuv;
mod usb_screen;
mod draw_bitmap;
mod clock;
mod draw_gif;
mod reboot;

#[cfg(feature = "usb-serial")]
fn main() -> Result<()>{
    // test_serial()?;

    // use reboot::reboot_serial;
    // reboot_serial()?;
    

    let usb_screens = find_usb_serial_device()?;

    if usb_screens.len() == 0{
        return Ok(());
    }

    let mut screen = serialport::new(&usb_screens[0].port_name, 115_200).open()?;

    let width = 160;
    let height = 128;
    // let width = 320;
    // let height = 240;

    draw_bitmap::draw(screen.as_mut(), width, height)?;

    sleep(Duration::from_secs(2));

    // clock::draw(screen.as_mut(), width, height)?;

    draw_gif::draw(screen.as_mut(), width, height)?;

    Ok(())
}

#[cfg(feature = "usb-raw")]
fn main() -> Result<()>{
    // use reboot::reboot_usb_raw;
    // reboot_usb_raw()?;

    println!("open usb usb screen...");
    let interface = usb_screen::open_usb_screen()?.unwrap();
    println!("open usb usb OK number:{}", interface.interface_number());

    let width = 160;
    let height = 128;

    // draw_bitmap::draw(&interface, width, height)?;

    // sleep(Duration::from_millis(2));

    // clock::draw(screen.as_mut(), width, height)?;

    draw_gif::draw(&interface, width, height)?;

    Ok(())
}

fn lz4test() -> Result<()> {
    use lz4_flex::compress_prepend_size;
    let img = open("./assets/rgb24.bmp")?.to_rgb8();
    println!("图像大小:{}x{}", img.width(), img.height());
    let rgb565 = rgb888_to_rgb565_le(&img, img.width() as usize, img.height() as usize);
    println!("rgb565:{}字节", rgb565.len());
    let result = compress_prepend_size(&rgb565);
    
    println!("压缩后:{}字节", result.len());

    std::fs::write("assets/127x64_le.lz4", &result)?;

    Ok(())
}

fn test_serial() -> Result<()>{
    let usb_screens = find_usb_serial_device()?;

    if usb_screens.len() == 0{
        return Ok(());
    }
    
    let mut screen = serialport::new(&usb_screens[0].port_name, 115_200).open()?;

    let img = open("./assets/320x240.png")?.to_rgb8();
    let t = Instant::now();

    for _ in 0..13{
        usb_screen::draw_rgb_image_serial(0, 0, &img, screen.as_mut())?;
    }

    println!("{}ms", t.elapsed().as_millis());
    Ok(())
}

fn test_usb() -> Result<()> {
    let interface = usb_screen::open_usb_screen()?.unwrap();

    let img = open("./assets/160x128.png")?.to_rgb8();
    let t = Instant::now();

    for _ in 0..40{
        usb_screen::draw_rgb_image(0, 0, &img, &interface)?;
    }
    println!("{}ms", t.elapsed().as_millis());

    Ok(())
}