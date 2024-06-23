use anyhow::Result;
use futures_lite::future::block_on;

use crate::usb_screen::{find_usb_serial_device, open_usb_screen, BULK_OUT_EP};

pub fn reboot_serial() -> Result<()>{
    let devices = find_usb_serial_device()?;

    println!("usb screen数量:{}", devices.len());

    if devices.len() == 0{
        return Ok(());
    }
    
    let mut screen = serialport::new(&devices[0].port_name, 115_200).open()?;

    const BOOT_USB:u64 = 7093010483740242786;
    screen.write(&BOOT_USB.to_be_bytes())?;
    Ok(())
}

pub fn reboot_usb_raw() -> Result<()>{
    let devices = open_usb_screen()?;

    if devices.is_none(){
        return Ok(());
    }

    let interface = devices.unwrap();
    
    const BOOT_USB:u64 = 7093010483740242786;
    block_on(interface.bulk_out(BULK_OUT_EP, BOOT_USB.to_be_bytes().into())).status?;
    Ok(())
}