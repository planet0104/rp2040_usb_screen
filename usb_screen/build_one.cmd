
:: 编译 st7789 240x240 的 USB Raw模式传输的uf2
cargo build --release --no-default-features --features "st7789-240x240,usb-raw,serial-num-7"
elf2uf2-rs .\target\thumbv6m-none-eabi\release\usb_screen .\uf2\usb_screen_240x240_USBRaw_sn7.uf2