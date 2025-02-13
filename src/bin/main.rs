#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::main;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Output;
use esp_hal::rng::Rng;
use esp_hal::time::Duration;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::{init, wifi};
use ieee80211::{data_frame::DataFrame, match_frames, mgmt_frame::DeauthenticationFrame};
use log::info;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let mut config = esp_hal::Config::default();
    config.cpu_clock = CpuClock::max();
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(144 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let mut led = Output::new(peripherals.GPIO7, esp_hal::gpio::Level::Low);

    let wifi = peripherals.WIFI;

    // We must initialize some kind of interface and start it.
    let (_, mut controller) = wifi::new_with_mode(&init, wifi, wifi::WifiApDevice).unwrap();
    controller.start().unwrap();

    let mut sniffer = controller.take_sniffer().unwrap();
    sniffer.set_promiscuous_mode(true).unwrap();
    sniffer.set_receive_cb(|packet| {
        let _ = match_frames! {
            packet.data,
            deauth = DeauthenticationFrame => {
                info!("deauth {} -> {} - reason: {:?}, rssi: {} dBm, noise floor: {} dBm, channel: {}", 
                    deauth.header.transmitter_address, 
                    deauth.header.receiver_address, 
                    deauth.reason, 
                    packet.rx_cntl.rssi, 
                    packet.rx_cntl.noise_floor, 
                    packet.rx_cntl.channel
                );
            }
            _ = DataFrame => {
                //info!("{} {} {}", packet.rx_cntl.rssi, packet.rx_cntl.noise_floor, packet.rx_cntl.channel); 
            }
        };
    });

    let delay = Delay::new();
    loop {
        let mut config = controller.configuration().unwrap();
        let mut channel = config.as_ap_conf_mut().channel;
        if channel > 5 {
            channel = 4;
        }
        channel += 1;
        config.as_ap_conf_mut().channel = channel;
        controller.set_configuration(&config).unwrap();
        led.toggle();
        delay.delay(Duration::millis(1000));
    }
}
