@REM cargo build --release
@REM elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen usb_screen_160x128.uf2

:: 编译 st7735 160x128 的 USB串口模式传输的uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-serial,serial-num-1"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBSerial_sn1.uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-serial,serial-num-2"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBSerial_sn2.uf2
:: 编译 st7735 160x128 的 USB Raw模式传输的uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-raw,serial-num-3"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBRaw_sn3.uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-raw,serial-num-4"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBRaw_sn4.uf2

:: 编译 st7789 240x320 的 USB串口模式传输的uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-serial,serial-num-5"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBSerial_sn5.uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-serial,serial-num-6"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBSerial_sn6.uf2
:: 编译 st7789 240x320 的 USB Raw模式传输的uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-raw,serial-num-7"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBRaw_sn7.uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-raw,serial-num-8"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBRaw_sn8.uf2