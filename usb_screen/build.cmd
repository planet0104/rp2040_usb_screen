@REM cargo build --release
@REM elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen usb_screen_160x128.uf2

:: 编译 st7735 160x128 的 USB串口模式传输的uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-serial"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBSerial.uf2
:: 编译 st7735 160x128 的 USB Raw模式传输的uf2
cargo build --release --no-default-features --features "st7735-128x160,usb-raw"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_160x128_USBRaw.uf2

:: 编译 st7789 240x320 的 USB串口模式传输的uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-serial"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBSerial.uf2
:: 编译 st7789 240x320 的 USB Raw模式传输的uf2
cargo build --release --no-default-features --features "st7789-240x320,usb-raw"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x320_USBRaw.uf2