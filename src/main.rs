//----------------------------------------------------------------------------
// @date 2024-04-09 14:03
// @author Martin Noblia
// TODOs
// - [x] sacar las dependencias que sobran
// - [x] conectar y hacer andar el lcd
// - [ ] hacer un menu para el lcd
//      - [ ] tiene que tener un modo para mostrar la hora
//          - [ ] Hacer una tarea que tenga a los botones que emitan una senial cuando cambian de
//          estado
//      - [ ] tiene que tener un modo para setear la hora
//      - [ ] tiene que tener un modo para setear la alarma
//      - [ ] tiene que tener un modo para alarma
//              - [ ] la alarma tiene que lanzar algun sonido(podria ser un buzzer para empezar)
//----------------------------------------------------------------------------
#![no_std]
#![no_main]

// use core::fmt::Write;
// use defmt::write;
// use core::fmt::Write;
use core::fmt::Write;
use defmt::info;
// use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Config};
use embassy_rp::rtc::{DateTime, DayOfWeek, Instance, Rtc};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex},
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use embassy_time::Timer;
use embassy_time::{Duration, Ticker};
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
type ChannelMutex = CriticalSectionRawMutex;

// Short-hand type alias for PubSubChannel
type Pub<T, const N: usize> = Publisher<'static, ChannelMutex, T, 1, N, 1>;
type Sub<T, const N: usize> = Subscriber<'static, ChannelMutex, T, 1, N, 1>;
type Ch<T, const N: usize> = PubSubChannel<ChannelMutex, T, 1, N, 1>;

const BUTTON_NUMBER: usize = 1;
pub type ButtonMessageType = bool;
pub type ButtonMessagePub = Pub<ButtonMessageType, BUTTON_NUMBER>;
pub type ButtonMessageSub = Sub<ButtonMessageType, BUTTON_NUMBER>;
pub static BUTTON_CHANNEL: Ch<ButtonMessageType, BUTTON_NUMBER> = PubSubChannel::new();

// TODO(elsuizo: 2024-07-12): ver como se puede hacer para que sea generico
#[embassy_executor::task]
pub async fn buttons_task(button1: AnyPin, button2: AnyPin, button_command: ButtonMessagePub) {
    // let mut ticker = Ticker::every(Duration::from_millis(200));
    info!("Hola desde la tarea buttons_tasks");
    let mut button1 = Input::new(button1, Pull::None);
    // let mut button2 = Input::new(button2, Pull::None);
    loop {
        button1.wait_for_high().await;
        button_command.publish_immediate(true);
        // ticker.next().await;
    }
}

// NOTE(elsuizo: 2024-07-12): esta seria la tarea que va a cambiar el estado del menu en el
// display, dependiendo desde que botton le llega
#[embassy_executor::task]
pub async fn state(mut button_command_input: ButtonMessageSub) {
    // let mut ticker = Ticker::every(Duration::from_millis(200));
    info!("Hola desde la tarea state");
    let mut counter = 0;
    loop {
        let message = button_command_input.next_message_pure().await;
        counter += 1;
        info!("recibimos {} mensaje !!!", counter);
        // ticker.next().await;
    }
}

pub fn clock_read<'r, T: Instance + 'r>(rtc: Rtc<'r, T>) -> String<256> {
    let mut time: String<256> = String::new();
    if let Ok(dt) = rtc.now() {
        info!(
            "aca dentro del if let tendriamos que mostrar la hora pero no se que pasa que no anda"
        );
        write!(
            &mut time,
            "{:02}:{:02}:{:02}",
            dt.hour, dt.minute, dt.second,
        )
        .unwrap();
    } else {
        info!("Parece que no anda el rtc")
    }
    time
}

#[embassy_executor::task]
pub async fn clock_update(mut button_command_input: ButtonMessageSub) {}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Comienzo de programa");
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);

    info!("Configuring the i2c");
    //-------------------------------------------------------------------------
    //                        display init
    //-------------------------------------------------------------------------
    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());

    let mut rtc = Rtc::new(p.RTC);

    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    led.set_high();
    display.init().ok();
    display.flush().ok();
    spawner.must_spawn(buttons_task(
        p.PIN_16.into(),
        p.PIN_19.into(),
        BUTTON_CHANNEL.publisher().unwrap(),
    ));
    spawner.must_spawn(state(BUTTON_CHANNEL.subscriber().unwrap()));
    //-------------------------------------------------------------------------
    //                        rtc init
    //-------------------------------------------------------------------------
    info!("Start RTC");
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
    // loop {
    //     Timer::after_millis(100).await;
    // }
}
