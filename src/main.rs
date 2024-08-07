//----------------------------------------------------------------------------
// @date 2024-04-09 14:03
// @author Martin Noblia
// TODOs
// - [x] sacar las dependencias que sobran
// - [x] conectar y hacer andar el lcd
// - [ ] hay que hacer un mod que tenga todo lo del clock
// - [ ] hay que hacer un mod que tenga todo lo del display
// - [ ] hay que hacer un mod que tenga todo lo de la ui
// - [ ] hacer un menu para el lcd
//      - [X] tiene que tener un modo para mostrar la hora
//          - [X] Hacer una tarea que tenga a los botones que emitan una senial cuando cambian de
//          estado
//      - [ ] ver como hacer para activar una alarma o hacerla a mano
//      - [ ] tiene que tener un modo para setear la hora
//          - [ ] esto tiene que llamar a una task `set-time` o algo asi
//      - [ ] tiene que tener un modo para setear la alarma
//          - [ ] esto tiene que llamar a una task `set-time` o algo asi
//      - [ ] tiene que tener un modo para alarma
//              - [ ] la alarma tiene que lanzar algun sonido(podria ser un buzzer para empezar)
//----------------------------------------------------------------------------
#![no_std]
#![no_main]
// TODO(elsuizo: 2024-07-25): put all the clock stuff in one file
mod clock;
use clock::Clock;
use clock::{ClockFSM, ClockState};
mod ui;
use embassy_rp::rtc::DayOfWeek;
use embassy_rp::rtc::{DateTime, Rtc};
use keypad::embedded_hal::digital::v2::InputPin;
use keypad::{keypad_new, keypad_struct};
use ui::Msg;
// use core::fmt::Write;
// use defmt::write;
// use core::fmt::Write;
// use core::fmt::Write;
use defmt::info;
// use defmt::*;
use embassy_executor::Spawner;
// use embassy_futures::select::{select, select3, Either};
use core::convert::Infallible;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::I2C1;
use embassy_rp::peripherals::RTC;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use embassy_time::{Duration, Ticker};
use heapless::{LinearMap, String};
use sh1106::{prelude::*, Builder};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_10X20, ascii::FONT_9X15, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

keypad_struct! {
    pub struct Keypad< Error = Infallible> {
        rows: (
            Input<'static, AnyPin>,
            Input<'static, AnyPin>,
            Input<'static, AnyPin>,
            Input<'static, AnyPin>,
        ),
        columns: (
            Output<'static, AnyPin>,
            Output<'static, AnyPin>,
            Output<'static, AnyPin>,
            Output<'static, AnyPin>,
        ),
    }
}

type ChannelMutex = CriticalSectionRawMutex;

// Short-hand type alias for PubSubChannel
type Pub<T, const N: usize> = Publisher<'static, ChannelMutex, T, 1, N, 1>;
type Sub<T, const N: usize> = Subscriber<'static, ChannelMutex, T, 1, N, 1>;
type Ch<T, const N: usize> = PubSubChannel<ChannelMutex, T, 1, N, 1>;

// NOTE(elsuizo: 2024-07-28): creo que esto es la cantidad de tasks que pueden recibir como
// parametro alguna de estas seniales
const BUTTONS_CHANNEL_CAP: usize = 2;
pub type ButtonMessageType = Msg;
pub type ButtonMessagePub = Pub<ButtonMessageType, BUTTONS_CHANNEL_CAP>;
pub type ButtonMessageSub = Sub<ButtonMessageType, BUTTONS_CHANNEL_CAP>;
pub static BUTTON_CHANNEL: Ch<ButtonMessageType, BUTTONS_CHANNEL_CAP> = PubSubChannel::new();

const CLOCK_CHANNEL_NUM: usize = 2;
pub type ClockMessageType = ClockState;
pub type ClockMessagePub = Pub<ClockMessageType, CLOCK_CHANNEL_NUM>;
pub type ClockMessageSub = Sub<ClockMessageType, CLOCK_CHANNEL_NUM>;
pub static CLOCK_STATE_CHANNEL: Ch<ClockMessageType, CLOCK_CHANNEL_NUM> = PubSubChannel::new();

#[embassy_executor::task]
pub async fn display(
    i2c: embassy_rp::i2c::I2c<'static, I2C1, embassy_rp::i2c::Blocking>,
    clock: Clock<'static, RTC>,
    mut clock_state_signal_in: ClockMessageSub,
) {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
    display.init().ok();
    display.flush().ok();

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();

    let logo_image = ImageRawLE::new(include_bytes!("../Images/rust.raw"), 64);

    loop {
        let time = clock.read();
        match clock_state_signal_in.next_message_pure().await {
            ClockState::Time => {
                Text::new(&time, Point::new(30, 13), normal).draw(&mut display);
            }
            ClockState::Image => {
                Image::new(&logo_image, Point::new(32, 0)).draw(&mut display);
            }
            ClockState::Alarm => {
                Text::new("Alarm!!!", Point::new(37, 13), normal).draw(&mut display);
            }
        }
        display.flush().ok();
        display.clear();
        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn buttons_reader(keypad: Keypad, button_command: ButtonMessagePub) {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    button_command.publish_immediate(Msg::Continue);
    let keys = keypad.decompose();

    // NOTE(elsuizo: 2024-08-05): esto es al pedo me parece...
    // let mut map: LinearMap<(usize, usize), Msg, 16> = LinearMap::new();
    // for (row_index, row) in keys.iter().enumerate() {
    //     for (col_index, _key) in row.iter().enumerate() {
    //         // info!("Pressed: ({}, {})", row_index, col_index);
    //         let indexs = (row_index, col_index);
    //         match indexs {
    //             (0, 0) => map.insert(indexs, Msg::One).unwrap(),
    //             (0, 1) => map.insert(indexs, Msg::Two).unwrap(),
    //             (0, 2) => map.insert(indexs, Msg::Three).unwrap(),
    //             (0, 3) => map.insert(indexs, Msg::A).unwrap(),
    //             (1, 0) => map.insert(indexs, Msg::Four).unwrap(),
    //             (1, 1) => map.insert(indexs, Msg::Five).unwrap(),
    //             (1, 2) => map.insert(indexs, Msg::Six).unwrap(),
    //             (1, 3) => map.insert(indexs, Msg::B).unwrap(),
    //             (2, 0) => map.insert(indexs, Msg::Seven).unwrap(),
    //             (2, 1) => map.insert(indexs, Msg::Eight).unwrap(),
    //             (2, 2) => map.insert(indexs, Msg::Nine).unwrap(),
    //             (2, 3) => map.insert(indexs, Msg::C).unwrap(),
    //             (3, 0) => map.insert(indexs, Msg::Asterisk).unwrap(),
    //             (3, 1) => map.insert(indexs, Msg::Zero).unwrap(),
    //             (3, 2) => map.insert(indexs, Msg::Numeral).unwrap(),
    //             (3, 3) => map.insert(indexs, Msg::D).unwrap(),
    //             (_, _) => panic!("Nooo"),
    //         };
    //     }
    // }

    // let first_key = &keys[0][0];
    loop {
        for (row_index, row) in keys.iter().enumerate() {
            for (col_index, key) in row.iter().enumerate() {
                if key.is_low().unwrap() {
                    // let button_pressed = map.get(&(row_index, col_index));
                    // button_command.publish_immediate(*button_pressed.unwrap());
                    match (row_index, col_index) {
                        (0, 0) => button_command.publish_immediate(Msg::One),
                        (0, 1) => button_command.publish_immediate(Msg::Two),
                        (0, 2) => {
                            let msg = Msg::Three;
                            button_command.publish_immediate(msg);
                            info!("mensaje: {}", msg);
                        }
                        (0, 3) => button_command.publish_immediate(Msg::A),
                        (1, 0) => button_command.publish_immediate(Msg::Four),
                        (1, 1) => button_command.publish_immediate(Msg::Five),
                        (1, 2) => button_command.publish_immediate(Msg::Six),
                        (1, 3) => button_command.publish_immediate(Msg::B),
                        (2, 0) => button_command.publish_immediate(Msg::Seven),
                        (2, 1) => button_command.publish_immediate(Msg::Eight),
                        (2, 2) => button_command.publish_immediate(Msg::Nine),
                        (2, 3) => button_command.publish_immediate(Msg::C),
                        (3, 0) => button_command.publish_immediate(Msg::Asterisk),
                        (3, 1) => button_command.publish_immediate(Msg::Zero),
                        (3, 2) => button_command.publish_immediate(Msg::Numeral),
                        (3, 3) => button_command.publish_immediate(Msg::D),
                        (_, _) => panic!("Nooo"),
                    }
                }
            }
        }
        button_command.publish(Msg::Continue).await;
        ticker.next().await;
    }
}

// NOTE(elsuizo: 2024-07-12): esta seria la tarea que va a cambiar el estado del menu en el
// display, dependiendo desde que botton le llega
#[embassy_executor::task]
pub async fn clock_controller(
    mut button_command_input: ButtonMessageSub,
    clock_state_signal_out: ClockMessagePub,
) {
    let mut ticker = Ticker::every(Duration::from_millis(30));
    let mut clock_fsm = ClockFSM::init(ClockState::Time);

    loop {
        let message = button_command_input.next_message_pure().await;
        clock_fsm.next_state(message);
        clock_state_signal_out.publish_immediate(clock_fsm.state);
        ticker.next().await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("init program");
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);
    let keypad = keypad_new!(Keypad {
        rows: (
            Input::new(AnyPin::from(p.PIN_0), Pull::Up),
            Input::new(AnyPin::from(p.PIN_1), Pull::Up),
            Input::new(AnyPin::from(p.PIN_2), Pull::Up),
            Input::new(AnyPin::from(p.PIN_3), Pull::Up),
        ),
        columns: (
            Output::new(AnyPin::from(p.PIN_4), Level::Low),
            Output::new(AnyPin::from(p.PIN_5), Level::Low),
            Output::new(AnyPin::from(p.PIN_6), Level::Low),
            Output::new(AnyPin::from(p.PIN_7), Level::Low),
        ),
    });
    info!("Configuring the i2c");
    //-------------------------------------------------------------------------
    //                        display init
    //-------------------------------------------------------------------------
    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());

    let rtc = Rtc::new(p.RTC);

    led.set_high();
    //-------------------------------------------------------------------------
    //                        rtc init
    //-------------------------------------------------------------------------
    info!("Start RTC");
    let now = DateTime {
        year: 2024,
        month: 7,
        day: 29,
        day_of_week: DayOfWeek::Monday,
        hour: 11,
        minute: 17,
        second: 0,
    };

    let clock = Clock::new(now, rtc).expect("Error creating the clock type");

    let _normal = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    spawner.must_spawn(clock_controller(
        BUTTON_CHANNEL.subscriber().unwrap(),
        CLOCK_STATE_CHANNEL.publisher().unwrap(),
    ));
    spawner.must_spawn(display(
        i2c,
        clock,
        CLOCK_STATE_CHANNEL.subscriber().unwrap(),
    ));
    spawner.must_spawn(buttons_reader(keypad, BUTTON_CHANNEL.publisher().unwrap()));
}

/// This is the principal function that renders all the menu states
pub fn draw_menu<D>(target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // normal text
    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();
    // text with background
    // let background = MonoTextStyleBuilder::from(&normal)
    //     .background_color(BinaryColor::On)
    //     .text_color(BinaryColor::Off)
    //     .build();

    Ok(())
}
