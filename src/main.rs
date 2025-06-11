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
use core::cell::{Cell, RefCell};
use embassy_sync::{channel, signal};
mod ui;
use defmt::info;
use embassy_rp::bind_interrupts;
use embassy_rp::rtc::DayOfWeek;
use embassy_rp::rtc::{DateTime, DateTimeFilter, Rtc};
use embassy_sync::pubsub::publisher;
use keypad::embedded_hal::digital::v2::InputPin;
use keypad::{keypad_new, keypad_struct};
use static_cell::StaticCell;
use ui::{show_menu, Msg};
// use defmt::*;
use embassy_executor::Spawner;
// use embassy_futures::select::{select, select3, Either};
use core::convert::Infallible;
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Config};
use embassy_rp::interrupt;
use embassy_rp::peripherals::I2C1;
use embassy_rp::peripherals::RTC;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher, Subscriber},
};
use embassy_time::{Duration, Ticker, Timer};
use heapless::String;
use sh1106::{prelude::*, Builder};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_9X15, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use portable_atomic::{AtomicBool, Ordering};

// TODO(elsuizo: 2024-08-09): ver como podemos sacar esto de aca ...
keypad_struct! {
    pub struct Keypad< Error = Infallible> {
        rows: (
            Input<'static>,
            Input<'static>,
            Input<'static>,
            Input<'static>,
        ),
        columns: (
            Output<'static>,
            Output<'static>,
            Output<'static>,
            Output<'static>,
        ),
    }
}

// use critical_section::Mutex;
/// Signal for notifying about state changes
static ALARM_TRIGGERED: signal::Signal<CriticalSectionRawMutex, ()> = signal::Signal::new();

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

// static GLOBAL_SHARED: Mutex<RefCell<Option<(Rtc<'static, RTC>, ClockFSM)>>> =
//     Mutex::new(RefCell::new(None));

const CLOCK_CHANNEL_NUM: usize = 2;
pub type ClockMessageType = (ClockState, String<256>);
pub type ClockMessagePub = Pub<ClockMessageType, CLOCK_CHANNEL_NUM>;
pub type ClockMessageSub = Sub<ClockMessageType, CLOCK_CHANNEL_NUM>;
pub static CLOCK_STATE_CHANNEL: Ch<ClockMessageType, CLOCK_CHANNEL_NUM> = PubSubChannel::new();

pub async fn alarm_sound_test<'a>(buzzer: &'a mut Output<'static>) {
    // TODO(elsuizo: 2024-08-17): hacer que esto sea un sonido real de alarma
    // quizas tambien tendriamos que hacer que sea infinita hasta que pase un evento
    buzzer.set_high();
    Timer::after(Duration::from_millis(300)).await;

    buzzer.set_low();
    Timer::after(Duration::from_millis(500)).await;

    buzzer.set_high();
    Timer::after(Duration::from_millis(100)).await;

    buzzer.set_low();
    Timer::after(Duration::from_millis(300)).await;
}

#[embassy_executor::task]
pub async fn show_display_states(
    i2c: embassy_rp::i2c::I2c<'static, I2C1, embassy_rp::i2c::Blocking>,
    mut clock_state_signal_in: ClockMessageSub,
    buzzer_pin: AnyPin,
) {
    // TODO(elsuizo: 2024-08-09): is that time ok???
    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
    display.init().ok();
    display.flush().ok();
    let mut buzzer = Output::new(buzzer_pin, Level::Low);

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();

    let _background = MonoTextStyleBuilder::from(&normal)
        .background_color(BinaryColor::On)
        .text_color(BinaryColor::Off)
        .build();

    let logo_image = ImageRawLE::new(include_bytes!("../Images/rust.raw"), 64);

    // TODO(elsuizo: 2024-08-13): this menus items should be a function...
    // TODO(elsuizo: 2025-06-08): we need the `time` variable only in few callings maybe is not a
    // good idea transmit over ...
    loop {
        match clock_state_signal_in.next_message_pure().await {
            (ClockState::DisplayTime, time) => {
                let _ = Text::new(&time, Point::new(30, 13), normal).draw(&mut display);
            }
            (ClockState::ShowImage, _) => {
                let _ = Image::new(&logo_image, Point::new(32, 0)).draw(&mut display);
            }
            // TODO(elsuizo: 2024-08-13): here should display the alarm date...
            (ClockState::DisplayAlarm, _) => {
                let _ = Text::new("Alarm!!!", Point::new(37, 13), normal).draw(&mut display);
            }
            // show the display menus
            (ClockState::Menu(a, b, c), _) => {
                show_menu(&mut display, (a, b, c)).expect("no se pudo mostrar ese estado");
            }
            (ClockState::TestSound, _) => {
                alarm_sound_test(&mut buzzer).await;
                info!("Alarm!!!");
            }
            (ClockState::SetTime, _time) => {
                let _ = Text::new("Settime under construction!!!", Point::new(37, 13), normal)
                    .draw(&mut display);
            }
            (ClockState::SetAlarm, _time) => {
                let _ = Text::new("SetAlarm under construction!!!", Point::new(37, 13), normal)
                    .draw(&mut display);
            }
            (ClockState::StopAlarm, _time) => {}
        }
        display.flush().ok();
        display.clear();
        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn keypad2msg(keypad: Keypad, button_command: ButtonMessagePub) {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    button_command.publish_immediate(Msg::Continue);
    let keys = keypad.decompose();

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
                            info!("message: {}", msg);
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
        button_command.publish(Msg::Continue).await; // no button was pressed
        ticker.next().await;
    }
}

// NOTE(elsuizo: 2024-07-12): esta seria la tarea que va a cambiar el estado del menu en el
// display, dependiendo desde que botton le llega
#[embassy_executor::task]
pub async fn clock_controller(
    mut button_command_input: ButtonMessageSub,
    clock_state_signal_out: ClockMessagePub,
    clock: Clock<'static, RTC>,
    mut clock_fsm: ClockFSM,
) {
    let mut ticker = Ticker::every(Duration::from_millis(30));

    loop {
        ALARM_TRIGGERED.wait().await;
        let time = clock.read();
        let message = button_command_input.next_message_pure().await;
        clock_fsm.next_state(message);
        clock_state_signal_out.publish_immediate((clock_fsm.state, time));
        ticker.next().await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("init program");
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);
    let buzzer = AnyPin::from(p.PIN_8);
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

    // let mut rtc = Rtc::new(p.RTC);
    led.set_high();
    //-------------------------------------------------------------------------
    //                        rtc init
    //-------------------------------------------------------------------------
    info!("Start RTC");
    let now = DateTime {
        year: 2025,
        month: 6,
        day: 10,
        day_of_week: DayOfWeek::Sunday,
        hour: 13,
        minute: 45,
        second: 0,
    };

    let alarm = DateTimeFilter {
        year: None,
        month: None,
        day_of_week: None,
        day: None,
        hour: Some(20),
        minute: Some(23),
        second: None,
    };

    let fsm = ClockFSM::init(ClockState::DisplayTime);

    let rtc = Rtc::new(p.RTC);
    let mut clock = Clock::new(now, rtc).expect("Error creating the clock type");
    // clock.set_alarm(alarm);

    spawner.must_spawn(clock_controller(
        BUTTON_CHANNEL.subscriber().unwrap(),
        CLOCK_STATE_CHANNEL.publisher().unwrap(),
        clock,
        fsm,
    ));
    spawner.must_spawn(show_display_states(
        i2c,
        CLOCK_STATE_CHANNEL.subscriber().unwrap(),
        buzzer,
    ));

    spawner.must_spawn(keypad2msg(keypad, BUTTON_CHANNEL.publisher().unwrap()));

    // Unmask the RTC IRQ so that the NVIC interrupt controller
    // will jump to the interrupt function when the interrupt occurs.
    // We do this last so that the interrupt can't go off while
    // it is in the middle of being configured
    unsafe {
        cortex_m::peripheral::NVIC::unmask(interrupt::RTC_IRQ);
    }
}

#[interrupt]
fn RTC_IRQ() {
    cortex_m::peripheral::NVIC::mask(interrupt::RTC_IRQ);

    ALARM_TRIGGERED.signal(());
}
