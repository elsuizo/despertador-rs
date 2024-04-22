//----------------------------------------------------------------------------
// @date 2024-04-09 14:03
// @author Martin Noblia
// TODOs
// - [ ] sacar las dependencias que sobran
// - [ ] conectar y hacer andar el lcd
// - [ ] hacer un menu para el lcd
//      - [ ] tiene que tener un modo para mostrar la hora
//      - [ ] tiene que tener un modo para setear la hora
//      - [ ] tiene que tener un modo para setear la alarma
//      - [ ] tiene que tener un modo para alarma
//              - [ ] la alarma tiene que lanzar algun sonido(podria ser un buzzer para empezar)
//----------------------------------------------------------------------------
#![no_std]
#![no_main]

// use core::fmt::Write;
// use defmt::write;
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::i2c::{self, Config};
use embassy_rp::rtc::{DateTime, DayOfWeek, Rtc};
use embassy_time::Timer;
use heapless::String;
use log::*;
use sh1106::{prelude::*, Builder};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_10X20, ascii::FONT_9X15, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    info!("Configuring the i2c");
    //-------------------------------------------------------------------------
    //                        display init
    //-------------------------------------------------------------------------
    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());

    let mut rtc = Rtc::new(p.RTC);

    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    display.init().ok();
    display.flush().ok();

    //-------------------------------------------------------------------------
    //                        rtc init
    //-------------------------------------------------------------------------
    // info!("Start RTC");
    let now = DateTime {
        year: 2024,
        month: 4,
        day: 18,
        day_of_week: DayOfWeek::Thursday,
        hour: 10,
        minute: 28,
        second: 0,
    };
    rtc.set_datetime(now).unwrap();

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();
    let mut time: String<256> = String::new();

    loop {
        // Text::new("Martin Noblia", Point::new(0, 20), normal)
        //     .draw(&mut display)
        //     .unwrap();
        // display.flush().ok();
        Timer::after_millis(500).await;

        if let Ok(dt) = rtc.now() {
            write!(
                &mut time,
                "{:02}:{:02}:{:02}",
                dt.hour, dt.minute, dt.second,
            )
            .unwrap();
            Text::new(&time, Point::new(0, 20), normal)
                .draw(&mut display)
                .unwrap();
            display.flush().ok();
            time.clear();
            display.clear();
            // info!(
            //     "Now: {}-{:02}-{:02} {}:{:02}:{:02}",
            //     dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second,
            // );
        }
    }
}
