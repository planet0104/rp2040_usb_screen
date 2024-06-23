use std::{any::Any, time::Instant};

use anyhow::Result;
use serialport::SerialPortType;

fn main() -> Result<()>{
    let ports = serialport::available_ports().unwrap_or(vec![]);
    let mut usb_screen = vec![];
    for p in ports {
        match p.port_type.clone(){
            SerialPortType::UsbPort(port) => {
                if port.serial_number.unwrap_or("".to_string()) == "62985215"{
                    usb_screen.push(p);
                    continue;
                }
            }
            _ => ()
        }
    }

    println!("usb screen数量:{}", usb_screen.len());

    if usb_screen.len() == 0{
        return Ok(());
    }
    
    let mut screen = serialport::new(&usb_screen[0].port_name, 115_200).open()?;

    const BOOT_USB:u64 = 7093010483740242786;
    screen.write(&BOOT_USB.to_be_bytes())?;
    Ok(())
}