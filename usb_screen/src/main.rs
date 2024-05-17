#![no_std]
#![no_main]

extern crate alloc;

use byte_slice_cast::AsMutByteSlice;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_13, PIN_14, PIN_4, PIN_6, PIN_7, SPI0, USB};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::spi::Spi;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embassy_usb::driver::{Endpoint, EndpointOut};
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Config};
use canvas::Canvas;
use core::slice;
use panic_halt as _;
mod st7735;
mod rgb565;
mod splash;
mod canvas;
use splash::Controller;
use embedded_alloc::Heap;
use st7735::{Orientation, ST7735};
// mod rgb2yuv;

// 这是一个随机生成的 GUID，允许 Windows 上的客户端找到我们的设备
const DEVICE_INTERFACE_GUIDS: &[&str] = &["{705E1599-5BFF-8DA9-6E33-7141B0636461}"];

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static CHANNEL: Channel<ThreadModeRawMutex, (i32, u16, u16, u16, u16), 1> = Channel::new();

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024*30;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
    
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("planet");
    config.product = Some("USB Screen");
    config.serial_number = Some("62985215");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut device_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    // Add the Microsoft OS Descriptor (MSOS/MOD) descriptor.
    // We tell Windows that this entire device is compatible with the "WINUSB" feature,
    // which causes it to use the built-in WinUSB driver automatically, which in turn
    // can be used by libusb/rusb software without needing a custom driver or INF file.
    // In principle you might want to call msos_feature() just on a specific function,
    // if your device also has other functions that still use standard class drivers.
    builder.msos_descriptor(windows_version::WIN8_1, 0);
    builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
    ));

    // Add a vendor-specific function (class 0xFF), and corresponding interface,
    // that uses our custom handler.
    let mut function = builder.function(0xFF, 0, 0);
    let mut interface = function.interface();
    let mut alt = interface.alt_setting(0xFF, 0, 0, None);
    let mut read_ep = alt.endpoint_bulk_out(64);
    let mut _write_ep = alt.endpoint_bulk_in(64);
    drop(function);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let image_data = &mut [0u16; 20_480];
    let image_data_clone = &mut [0u16; 20_480];
    let mut pix_index = 0;
    let mut image_start = false;
    let mut image_width = 0u16;
    let mut image_height = 0u16;
    let mut image_x = 0u16;
    let mut image_y = 0u16;

    const IMAGE_AA:u64 = 7596835243154170209;
    const BOOT_USB:u64 = 7093010483740242786;
    const IMAGE_BB:u64 = 7596835243154170466;
    const MAGIC_NUM_LEN: usize = 8;
    let magic_num_buf = &mut [0u8; MAGIC_NUM_LEN];

    //屏幕缓冲区指针
    let image_data_clone_ptr = image_data_clone.as_ptr();
    let _ = spawner.spawn(display_task(p.SPI0, p.PIN_6, p.PIN_7, p.PIN_4, p.PIN_13, p.PIN_14, p.DMA_CH0, p.DMA_CH1, image_data_clone_ptr as i32));

    let echo_fut = async {
        loop {
            read_ep.wait_enabled().await;
            loop {
                let mut data = [0; 64];
                match read_ep.read(&mut data).await {
                    Ok(n) => {
                        magic_num_buf.copy_from_slice(&data[0..MAGIC_NUM_LEN]);
                        let magic_num = u64::from_be_bytes(*magic_num_buf);
                        
                        if magic_num == IMAGE_AA{
                            //图像开始
                            image_width = u16::from_be_bytes([data[MAGIC_NUM_LEN], data[MAGIC_NUM_LEN+1]]);
                            image_height = u16::from_be_bytes([data[MAGIC_NUM_LEN+2], data[MAGIC_NUM_LEN+3]]);
                            image_x = u16::from_be_bytes([data[MAGIC_NUM_LEN+4], data[MAGIC_NUM_LEN+5]]);
                            image_y = u16::from_be_bytes([data[MAGIC_NUM_LEN+6], data[MAGIC_NUM_LEN+7]]);
                            image_start = true;
                            pix_index = 0;
                        }else if magic_num == IMAGE_BB{
                            //图像结束
                            image_start = false;
                            let len: usize = (image_width * image_height) as usize;
                            image_data_clone[0..len].copy_from_slice(&image_data[0..len]);
                            let image_data_clone_ptr = image_data_clone.as_ptr();
                            //异步绘制
                            let _res = CHANNEL.sender().send((image_data_clone_ptr as i32, image_x, image_y, image_width, image_height)).await;
                            // let mut text: String<64> = String::new();
                            // let _ = writeln!(&mut text, "{:?}", _res);
                            // let _ = _write_ep.write(text.as_bytes()).await;
                            //这里sleep，接收一帧的时间从80ms提升到60ms
                            Timer::after_millis(1).await;
                        }else if magic_num == BOOT_USB{
                            reset_to_usb_boot(0, 0);
                        }else{
                            //图像传输中
                            //像素(u16)数量
                            let pix_num = n/2;
                            if pix_index+pix_num > image_data.len() || !image_start{
                                continue;
                            }
                            let pix_slice = &mut image_data[pix_index..pix_index+pix_num];
                            let pix_slice_ptr = pix_slice.as_mut_byte_slice();
                            pix_slice_ptr.copy_from_slice(&data[0..n]);
                            pix_index += pix_num;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, echo_fut).await;
}

#[embassy_executor::task]
async fn display_task(spi: SPI0, p6: PIN_6, p7: PIN_7, p4: PIN_4, p13: PIN_13, p14: PIN_14, dma_ch0: DMA_CH0, dma_ch1: DMA_CH1, image_buf_ptr: i32) {
    /*
    GND <=> GND
    VCC <=> 3V3
    SCL <=> SCLK(GPIO6)
    SDA <=> MOSI(GPIO7)
    RES <=> RST(GPIO14)
    DC  <=> DC(GPIO13)
    CS  <=> GND
    BLK <=> 不连接
     */

    let mut splash = Controller::new(RoscRng);
    let mut frame_received = false;

    let spi_sclk = p6;
    let spi_mosi = p7;
    let spi_miso = p4;
    let mut spi_cfg = embassy_rp::spi::Config::default();
    spi_cfg.frequency = 64_000_000u32;
    let spi = Spi::new(spi, spi_sclk, spi_mosi, spi_miso, dma_ch0, dma_ch1, spi_cfg);

    let dc = Output::new(p13, Level::Low);
    let rst: Output<PIN_14> = Output::new(p14, Level::Low);
    let screen_width = 128;
    let screen_height = 160;
    let mut disp = ST7735::new(spi, dc, Some(rst), true, false, screen_width, screen_height);
    disp.init().await.unwrap();
    disp.set_orientation(&Orientation::Landscape).await.unwrap();

    //用于绘图的缓冲区
    let buf = unsafe { slice::from_raw_parts_mut(image_buf_ptr as *mut u16, 20_480) };
    let mut canvas = Canvas{ buf, width: 160, height: 128 };

    loop {
        let frame = if !frame_received{
            match CHANNEL.try_receive(){
                Ok(ret) => {
                    frame_received = true;
                    ret
                }
                Err(_) => {
                    splash.update();
                    splash.render(&mut canvas);
                    disp.draw_image_at(0, 0, &canvas.buf, 160).await;
                    continue;
                }
            }
        }else{
            CHANNEL.receive().await
        };

        let (ptr, x, y, width, height) = frame;
        let len = (width * height) as usize;
        let img = unsafe { slice::from_raw_parts(ptr as *mut u16, 20_480) };
        let image = &img[0..len];
        disp.draw_image_at(x, y, &image, width).await;
    }
}