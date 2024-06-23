#![no_std]
#![no_main]

#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]

extern crate alloc;

use core::mem::MaybeUninit;
use alloc::vec;
use alloc::vec::Vec;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{PIN_13, PIN_14, PIN_4, PIN_6, PIN_7, SPI0, USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_alloc::Heap;
use static_cell::StaticCell;
#[cfg(feature = "st7735-128x160")]
mod st7735;
#[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
mod st7789;
// mod rgb2yuv;
mod rgb565;
mod splash;
#[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
mod resize;
use panic_halt as _;

pub const DISPLAY_FREQ: u32 = 64_000_000;

type ImageInfo = (Vec<u8>, u16, u16, u16, u16);

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static USB_CHANNEL: Channel<CriticalSectionRawMutex, ImageInfo, 1> = Channel::new();
#[allow(dead_code)]
static DRAW_CHANNEL: Channel<CriticalSectionRawMutex, u16, 1> = Channel::new();

//是否正在绘制中，正在绘制屏幕时，限制接收缓冲区最多为1帧，否则会会内存溢出。
#[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
static DISPLAY_LOCK: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, core::cell::RefCell<bool>> = embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(false));

#[global_allocator]
static HEAP: Heap = Heap::empty();

//图像传输开始标记(8字节)
const IMAGE_AA:u64 = 7596835243154170209;
//图像传输结束标记(8字节)
const IMAGE_BB:u64 = 7596835243154170466;
//重启到U盘模式命令(8字节)
const BOOT_USB:u64 = 7093010483740242786;
const MAGIC_NUM_LEN: usize = 8;

//embassy-executor使用12K, 堆内存使用剩余内存
const HEAP_SIZE: usize = 1024*226; //经过测试200K内存不足够解压320x240的lz4图像
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

pub fn test_runner(_test: &[&dyn Fn()]) {
    loop {}
}

#[cortex_m_rt::entry]
fn main() -> ! {
    {
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    /*

        ========================= 不同的压缩方式传输和解压耗时对比 ==============
        绘制耗时单独：160x128 耗时: 按行绘制26ms / 满屏绘制 22ms 都可满足30帧以上绘制
        Less解压缩单独测试：160x128 耗时: 25ms / 320x80 31ms (完全解压320x240需要90ms，速度太慢)
        JPG解压缩耗时(只能在core1)：64x32 2.16k 解压缩耗时12ms / 160x128 9.27k 解压76ms
        YUV数据解压耗时: 160x128 30K 耗时29ms
        zune-inflate解压: 160x128 24K(27K) 耗时 49ms
        lz4_flex解压: 160x128 9K 耗时 8ms / 320x240: 16.3K 耗时 18ms
        
        YUV图像大小: 320x240:112.5K / 160x128: 30K / 320x80: 38K
        Less压缩大小: 320x240:83.4K / 160x128: 28.4K / 320x80: 30.15K
        deflate压缩大小: 160x128: 24.2K / 320x240: 68.1K
        lz4压缩大小: 160x128:9K / 320x240: 16.3K
        RGB565大小: 320x240: 150K / 160x128: 40K
        USB纯数据传输速度: 512K/S ( 1M速度为 2048ms ) / 传输160x128的RGB565 12.8帧 / 160x128 lz4 56.8帧 / 传输320x240的RGB565 3.4帧 / 320x240 lz4 31.4帧
        USB Serial最快传输速度: 512K/S

        [直接传输RGB565格式速度]
        160x128 每帧40K，速度12帧
        320x240 每帧150K，速度3.4帧
        [直接传输RGB565内存占用]
        160x128: 接收缓冲区40K + 40K缓冲区。
        320x240: 接收缓冲区50K + 150K绘图缓冲区。

        [使用lz4压缩格式速度]
        160x128 每帧9K，速度>56帧（传输18ms/解压8ms/绘制26ms） core0解压，core1绘制速度最快，可超过30帧。
        320x240 每帧16.3K，速度>31.4帧（传输32ms/解压18ms/绘制?ms）
        [直接传输RGB565内存占用]
        160x128: 接收缓冲区40K + 40K缓冲区。
        320x240: 接收缓冲区50K + 150K绘图缓冲区。
        
        [数据传输过程]
        1、图像开始后，清空接收缓冲区，准备接收数据
        2、每接收到50K图像后，将数据移动到core2线程中，并复制到绘图缓冲区。
        3、图像结束后，发送消息给绘图缓冲区，开始绘制。
     */

    let p = embassy_rp::init(Default::default());

    
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                #[cfg(feature = "st7735-128x160")]
                {
                    spawner.spawn(core1_task(p.SPI0, p.PIN_6, p.PIN_7, p.PIN_4, p.PIN_13, p.PIN_14, p.DMA_CH0, p.DMA_CH1)).unwrap();
                }
                #[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
                {
                    spawner.spawn(core1_task(p.SPI0, p.PIN_6, p.PIN_7, p.PIN_4, p.PIN_13, p.PIN_14, p.PIN_9)).unwrap();
                }
            });
        },
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    #[cfg(feature = "usb-raw")]
    executor0.run(|spawner| spawner.spawn(core0_task_usb_raw(p.USB, spawner.clone())).unwrap());

    #[cfg(feature = "usb-serial")]
    executor0.run(|spawner| spawner.spawn(core0_task_usb_serial(p.USB, spawner.clone())).unwrap());

}

//通过USB serial传输数据
#[cfg(feature = "usb-serial")]
#[embassy_executor::task]
async fn core0_task_usb_serial(usb: USB, spawner: Spawner) {
    use embassy_usb::class::cdc_acm::{CdcAcmClass, State};

    // Create the driver, from the HAL.
    let driver = Driver::new(usb, Irqs);

    // Create embassy-usb Config
    let config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("planet");
        config.product = Some("USB Screen");
        config.serial_number = Some("62985215");
        config.max_power = 500;
        config.max_packet_size_0 = 64;

        // Required for windows compatibility.
        // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
        config.device_class = 0xEF;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
        static DECICE_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        
        let builder = embassy_usb::Builder::new(
            driver,
            config,
            DECICE_DESCRIPTOR.init([0; 256]),
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    // Run the USB device.
    spawner.spawn(usb_task(usb)).unwrap();

    // 接收串口数据

    let magic_num_buf = &mut [0u8; MAGIC_NUM_LEN];
    let mut image_width = 0;
    let mut image_height = 0;
    let mut image_x = 0;
    let mut image_y = 0;

    //接收到的数据
    let mut image_buf = vec::Vec::new();
    
    loop {
        class.wait_connection().await;
        
        let mut buf = [0; 64];

        loop {
            let ret = class.read_packet(&mut buf).await;
            if ret.is_err(){
                continue;
            }
            let len = ret.unwrap();
            let data = &buf[..len];

            //串口数据有可能出错
            if len < MAGIC_NUM_LEN{
                continue;
            }
            
            magic_num_buf.copy_from_slice(&data[0..MAGIC_NUM_LEN]);
            let magic_num = u64::from_be_bytes(*magic_num_buf);
            
            if magic_num == IMAGE_AA{
                image_width = u16::from_be_bytes([data[MAGIC_NUM_LEN], data[MAGIC_NUM_LEN+1]]);
                image_height = u16::from_be_bytes([data[MAGIC_NUM_LEN+2], data[MAGIC_NUM_LEN+3]]);
                image_x = u16::from_be_bytes([data[MAGIC_NUM_LEN+4], data[MAGIC_NUM_LEN+5]]);
                image_y = u16::from_be_bytes([data[MAGIC_NUM_LEN+6], data[MAGIC_NUM_LEN+7]]);
                image_buf.clear();
            }else if magic_num == IMAGE_BB{
                #[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
                {
                    loop{
                        if let Ok(mut lock) = DISPLAY_LOCK.try_lock(){
                            if *lock.get_mut() == false{
                                break;
                            }
                        }
                        embassy_time::Timer::after_millis(1).await;
                    }
                    USB_CHANNEL.send((image_buf.clone(), image_x, image_y, image_width, image_height)).await;
                    image_buf.clear();
                }
                
                #[cfg(feature = "st7735-128x160")]
                {
                    if let Ok(image) = lz4_flex::decompress_size_prepended(&image_buf){
                        USB_CHANNEL.send((image, image_x, image_y, image_width, image_height)).await;
                    }
                    image_buf.clear();
                }
            }else if magic_num == BOOT_USB{
                reset_to_usb_boot(0, 0);
            }else{
                //图像传输中
                if image_buf.len() <320*240*2{
                    image_buf.extend_from_slice(data);
                }
            }
            //返回字符串
            // let msg = format!("{total}");
            // let _ = class.write_packet(msg.as_byte_slice()).await;
        }
    }
}

#[cfg(feature = "usb-serial")]
type MyUsbDriver = Driver<'static, USB>;
#[cfg(feature = "usb-serial")]
type MyUsbDevice = embassy_usb::UsbDevice<'static, MyUsbDriver>;

#[cfg(feature = "usb-serial")]
#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

// 通过USB Raw Bulk接收数据
#[cfg(feature = "usb-raw")]
#[embassy_executor::task]
async fn core0_task_usb_raw(usb: USB, _spawner: Spawner) {
    use embassy_usb::driver::{Endpoint, EndpointOut};
    use embassy_usb::Config;
    use embassy_usb::Builder;
    use embassy_usb::msos::windows_version;
    use futures::future::join;
    
    //-------------- 初始化 usb ------------------------
    
    // Create the driver, from the HAL.
    let driver = Driver::new(usb, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("planet");
    config.product = Some("USB Screen");
    config.serial_number = Some("62985215");
    config.max_power = 500;
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

    // 这是一个随机生成的 GUID，允许 Windows 上的客户端找到我们的设备
    const DEVICE_INTERFACE_GUIDS: &[&str] = &["{705E1599-5BFF-8DA9-6E33-7141B0636461}"];

    // Add the Microsoft OS Descriptor (MSOS/MOD) descriptor.
    // We tell Windows that this entire device is compatible with the "WINUSB" feature,
    // which causes it to use the built-in WinUSB driver automatically, which in turn
    // can be used by libusb/rusb software without needing a custom driver or INF file.
    // In principle you might want to call msos_feature() just on a specific function,
    // if your device also has other functions that still use standard class drivers.
    builder.msos_descriptor(windows_version::WIN8_1, 0);
    builder.msos_feature(embassy_usb::msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(embassy_usb::msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        embassy_usb::msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
    ));

    // Add a vendor-specific function (class 0xFF), and corresponding interface,
    // that uses our custom handler.
    let mut function = builder.function(0xFF, 0, 0);
    let mut interface = function.interface();
    let mut alt = interface.alt_setting(0xFF, 0, 0, None);
    let mut read_ep = alt.endpoint_bulk_out(64);
    #[allow(unused_mut, unused_variables)]
    let mut write_ep = alt.endpoint_bulk_in(64);
    drop(function);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let magic_num_buf = &mut [0u8; MAGIC_NUM_LEN];
    let mut image_width = 0;
    let mut image_height = 0;
    let mut image_x = 0;
    let mut image_y = 0;

    //接收到的数据
    let mut buf = vec::Vec::new();

    //图像接收任务
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
                            //清空数据
                            buf.clear();

                            //往回传输数据(往USB写入数据后，USB主机必须读取，否则会导致程序卡死)
                            // let msg = format!("rev:{image_x}x{image_y} {image_width}x{image_height}");
                            // write_ep.write(msg.as_byte_slice()).await.ok();
                        }else if magic_num == IMAGE_BB{

                            //240x320屏幕占用内存较大，绘制的同时再解压数据内存不够用（150K*2），所以仅缓存一次接收到的压缩数据
                            //等待core1解压绘制完成后，再发送新的压缩帧，达到12帧左右的速度
                            #[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
                            {
                                //如果正在绘制中，等待绘制完成
                                loop{
                                    if let Ok(mut lock) = DISPLAY_LOCK.try_lock(){
                                        if *lock.get_mut() == false{
                                            break;
                                        }
                                    }
                                    embassy_time::Timer::after_millis(1).await;
                                }
                                //压缩图像结束，发送数据到core1线程
                                USB_CHANNEL.send((buf.clone(), image_x, image_y, image_width, image_height)).await;
                                //清空数组
                                buf.clear();
                            }
                            
                            //160x128屏幕，在core0解压，core1绘制速度最快
                            #[cfg(feature = "st7735-128x160")]
                            {
                                let image = lz4_flex::decompress_size_prepended(&buf).unwrap();
                                buf.clear();
                                //解压后的图像结束，发送数据到core1线程
                                USB_CHANNEL.send((image, image_x, image_y, image_width, image_height)).await;
                            }

                        }else if magic_num == BOOT_USB{
                            reset_to_usb_boot(0, 0);
                        }else{
                            //图像传输中
                            if buf.len() <320*240*2{
                                buf.extend_from_slice(&data[0..n]);
                            }
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

#[cfg(feature = "st7735-128x160")]
#[embassy_executor::task]
async fn core1_task(spi: SPI0, p6: PIN_6, p7: PIN_7, p4: PIN_4, p13: PIN_13, p14: PIN_14, dma_ch0: embassy_rp::peripherals::DMA_CH0, dma_ch1: embassy_rp::peripherals::DMA_CH1) {
    use byte_slice_cast::AsByteSlice;


    let mut display_manager = st7735::ST7735DisplayManager::new(spi, p6, p7, p4, p13, p14, dma_ch0, dma_ch1).await.unwrap();

    let mut splash = splash::Controller::new(embassy_rp::clocks::RoscRng);
    let mut frame_received = false;
    //在没有图像传输时，绘制吃豆人图像
    let mut canvas = splash::Canvas::new();

    loop {
        //没有接收到任何图像时，循环绘制吃豆人
        let (image, x, y, width, height) = if !frame_received{
            match USB_CHANNEL.try_receive(){
                Ok(ret) => {
                    frame_received = true;
                    ret
                }
                Err(_) => {
                    splash.update();
                    splash.render(&mut canvas);
                    display_manager.display_image_be(canvas.buf.as_byte_slice(), 0, 0, canvas.width as u16, canvas.height as u16).await;
                    continue;
                }
            }
        }else{
            //一旦从USB接收到图像，就一直等待图像到达
            USB_CHANNEL.receive().await
        };
        
        //绘制
        //全屏绘制可达到40帧左右(core0解压)
        display_manager.display_image_be(&image, x, y, width, height).await;
        //释放内存
        drop(image);
    }
}

//参考代码：https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/spi_display.rs
#[cfg(any(feature = "st7789-240x320", feature = "st7789-240x240"))]
#[embassy_executor::task]
async fn core1_task(spi: SPI0, p6: PIN_6, p7: PIN_7, p4: PIN_4, p13: PIN_13, p14: PIN_14, display_cs: embassy_rp::peripherals::PIN_9) {
    use core::{cell::RefCell, f32::consts::PI};

    use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
    use embassy_rp::{clocks::RoscRng, gpio::{Level, Output}, spi::{self, Blocking, Spi}};
    use embassy_time::{Duration, Timer};
    use rgb565::Rgb565Pixel;
    use splash::utils::random_usize;
    use st7789::{interface::SPIDeviceInterface, Orientation, ST7789};
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use embassy_sync::blocking_mutex::Mutex;
    use micromath::F32Ext;

    /*
    GND
    VCC
    SCL > clk (PIN6)
    SDA > mosi (PIN7)
    RESET > rst (PIN14)
    AO > dc(PIN13)
    CS > cs(PIN9)
    BL > bl(VCC)
    */
    
    let spi_sclk = p6;
    let spi_mosi = p7;
    let spi_miso = p4;
    let dc = Output::new(p13, Level::Low);
    let rst: Output<PIN_14> = Output::new(p14, Level::Low);

    // create SPI
    let mut display_config = spi::Config::default();
    display_config.frequency = DISPLAY_FREQ;
    // display_config.phase = spi::Phase::CaptureOnSecondTransition;
    // display_config.polarity = spi::Polarity::IdleHigh;

    let touch_config = spi::Config::default();

    let spi: Spi<'_, _, Blocking> = Spi::new_blocking(spi, spi_sclk, spi_mosi, spi_miso, touch_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let display_spi = SpiDeviceWithConfig::new(&spi_bus, Output::new(display_cs, Level::High), display_config);
    
    // display interface abstraction from SPI and DC
    let di = SPIDeviceInterface::new(display_spi, dc);


    // create driver
    #[cfg(feature = "st7789-240x320")]
    let mut display = ST7789::new(di, rst, 240, 320);
    #[cfg(feature = "st7789-240x240")]
    let mut display = ST7789::new(di, rst, 240, 240);

    #[cfg(feature = "st7789-240x320")]
    let screen_width = 320;
    #[cfg(feature = "st7789-240x320")]
    let screen_height = 240;
    #[cfg(feature = "st7789-240x240")]
    let screen_width = 240;
    #[cfg(feature = "st7789-240x240")]
    let screen_height = 240;

    // initialize
    display.init().await.unwrap();
    display.set_orientation(Orientation::Landscape).unwrap();
    st7789::interface::clear_rect(&mut display, Rgb565Pixel::from_rgb(0, 0, 0).0, 0, 0, screen_width, screen_height);
    
    let mut frame_received = false;
    let mut clear = false;
    let mut t = 3.;
    let mut d = 100.;
    let mut cx = 160.;
    let mut cy = 120.;
    let mut scale = 5.0;
    let mut depress = 5.0;

    loop {
        //没有接收到任何图像时，循环绘制图案
        let (compressed, x, y, width, height) = if !frame_received{
            match USB_CHANNEL.try_receive(){
                Ok(ret) => {
                    frame_received = true;
                    ret
                }
                Err(_) => {
                    if !clear{
                        t = random_usize(&mut RoscRng, 3, 8) as f32;
                        d = random_usize(&mut RoscRng, 40, 100) as f32;
                        cx = random_usize(&mut RoscRng, 150, 200) as f32;
                        cy = random_usize(&mut RoscRng, 100, 120) as f32;
                        scale = random_usize(&mut RoscRng, 4, 10) as f32;
                        depress = random_usize(&mut RoscRng, 3, 8) as f32;
                    }
                    let r = random_usize(&mut RoscRng, 20, 255) as u8;
                    let g = random_usize(&mut RoscRng, 20, 255) as u8;
                    let b = random_usize(&mut RoscRng, 20, 255) as u8;
                    let white = Rgb565Pixel::from_rgb(r, g, b).0;
                    let black = Rgb565Pixel::from_rgb(0, 0, 0).0;

                    let color = if clear{
                        black
                    }else{
                        white
                    };

                    let mut a = 0.0;
                    while a < PI * 2.0 {
                        let b = d + d / depress * (3.0 * t * a).sin();
                        let c = b * (1.0 / 2.0 + 1.0 / 2.0 * (t * a).sin());

                        let x = cx + c * a.cos() * scale / 5.0;
                        let y = cy - c * a.sin();
                        
                        st7789::interface::clear_rect(&mut display, color, x as u16, y as u16, 1, 1);

                        a += PI / (80.0 * t);
                    }
                    clear = !clear;
                    if !clear{
                        // Timer::after(Duration::from_secs(1)).await;
                    }else{
                        Timer::after(Duration::from_secs(3)).await;
                    }
                    continue;
                }
            }
        }else{
            //一旦从USB接收到图像，就一直等待图像到达
            USB_CHANNEL.receive().await
        };

        let mut lock = DISPLAY_LOCK.lock().await;
        *lock.get_mut() = true;

        //解压 如果是串口传输，有可能出现错误帧，这里要进行判断
        let image = match lz4_flex::decompress_size_prepended(&compressed){
            Err(_err) => {
                *lock.get_mut() = false;
                drop(lock);
                continue;
            }
            Ok(image) => image
        };
        drop(compressed);

        //调用draw_rgb565_u8速度最快，使用Big-Endian
        st7789::interface::draw_rgb565_u8(&mut display, &image, x, y, width, height);
        //释放内存
        drop(image);
        // DRAW_CHANNEL.send(0).await;
        *lock.get_mut() = false;
        drop(lock);
    }
}