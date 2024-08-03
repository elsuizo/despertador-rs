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
// mod clock;
// use clock::clock_update;
use core::cell::{Cell, RefCell};
use embassy_executor::{Executor, InterruptExecutor};
// use core::fmt::Write;
// use defmt::write;
// use core::fmt::Write;
use core::fmt::Write;
use defmt::info;
use static_cell::{ConstStaticCell, StaticCell};
// use defmt::*;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_futures::select::{select, select3, Either};
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::i2c::I2c;
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::I2C1;
use embassy_rp::peripherals::RTC;
use embassy_rp::rtc::{DateTime, DayOfWeek, Instance, Rtc};
use embassy_sync::{blocking_mutex, mutex};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex, RawMutex},
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

#[derive(Debug, Clone, Copy)]
pub enum ClockState {
    Time,
    Alarm,
    Image,
}

#[derive(Copy, Clone)]
pub enum Msg {
    Up,       // Up button
    Down,     // Down button
    Continue, // Continue in the actual state
}

#[derive(Clone, Debug)]
pub struct ClockFSM {
    pub state: ClockState,
}

impl ClockFSM {
    pub fn init(state: ClockState) -> Self {
        Self { state }
    }

    pub fn next_state(&mut self, msg: Msg) {
        use ClockState::*;
        use Msg::*;

        self.state = match (self.state, msg) {
            (Time, Up) => Alarm,
            (Time, Continue) => Time,
            (Alarm, Down) => Time,
            (Alarm, Up) => Image,
            (Alarm, Continue) => Alarm,
            (Time, Down) => Image,
            (Image, _) => Time,
            // (Image, Continue) => Image,
        }
    }
}

#[embassy_executor::task]
pub async fn display(
    i2c: embassy_rp::i2c::I2c<'static, I2C1, embassy_rp::i2c::Blocking>,
    rtc: embassy_rp::rtc::Rtc<'static, RTC>,
    mut clock_state_signal_in: ClockMessageSub,
) {
    let mut ticker = Ticker::every(Duration::from_millis(300));
    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
    display.init().ok();
    display.flush().ok();

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();

    let logo_image = ImageRawLE::new(include_bytes!("../Images/rust.raw"), 64);

    loop {
        let time = clock_read(&rtc);
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

fn clock_read<'r, T: Instance + 'r>(rtc: &Rtc<'r, T>) -> String<256> {
    let mut time: String<256> = String::new();
    if let Ok(dt) = rtc.now() {
        write!(
            &mut time,
            "{:02}:{:02}:{:02}\n{:}-{:}-{:}\n{:?}",
            dt.hour, dt.minute, dt.second, dt.day, dt.month, dt.year, dt.day_of_week
        )
        .unwrap();
    } else {
        info!("The RTC is not working ...")
    }
    time
}

#[embassy_executor::task]
pub async fn buttons_reader(button1: AnyPin, button2: AnyPin, button_command: ButtonMessagePub) {
    let mut ticker = Ticker::every(Duration::from_millis(30));
    // info!("Hola desde la tarea buttons_tasks");
    let mut button1 = Input::new(button1, Pull::Up);
    let mut button2 = Input::new(button2, Pull::Up);
    button_command.publish_immediate(Msg::Continue);
    loop {
        match select(button1.wait_for_low(), button2.wait_for_low()).await {
            Either::First(_) => button_command.publish_immediate(Msg::Up),
            Either::Second(_) => button_command.publish_immediate(Msg::Down),
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

    info!("Configuring the i2c");
    //-------------------------------------------------------------------------
    //                        display init
    //-------------------------------------------------------------------------
    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());

    let mut rtc = Rtc::new(p.RTC);

    // let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    led.set_high();
    // display.init().ok();
    // display.flush().ok();
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
    rtc.set_datetime(now).unwrap();

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    spawner.must_spawn(clock_controller(
        BUTTON_CHANNEL.subscriber().unwrap(),
        CLOCK_STATE_CHANNEL.publisher().unwrap(),
    ));
    spawner.must_spawn(display(i2c, rtc, CLOCK_STATE_CHANNEL.subscriber().unwrap()));
    spawner.must_spawn(buttons_reader(
        p.PIN_16.into(),
        p.PIN_17.into(),
        BUTTON_CHANNEL.publisher().unwrap(),
    ));
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
