use crate::ui::Msg;
use crate::String;
use core::fmt::Write;
use defmt::info;
use embassy_rp::peripherals::RTC;
use embassy_rp::rtc::{DateTime, DateTimeFilter, DayOfWeek, Instance, Rtc, RtcError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum ClockState {
    DisplayTime,
    SetTime,
    DisplayAlarm,
    SetAlarm,
    ShowImage,
    Menu(bool, bool, bool), // menu rows
                            // Menu,
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
            (DisplayTime, A) => DisplayAlarm,
            (DisplayTime, Asterisk) => SetTime,
            (DisplayTime, Continue) => DisplayTime,
            (DisplayTime, Numeral) => ShowImage,
            (ShowImage, Continue) => ShowImage,
            (DisplayTime, B) => Menu(false, false, false),
            (DisplayTime, _) => DisplayTime, // any other keys
            // up
            (Menu(true, false, false), A) => Menu(false, false, true),
            (Menu(true, false, false), Continue) => Menu(true, false, false),

            (Menu(false, true, false), A) => Menu(true, false, false),
            (Menu(false, true, false), Continue) => Menu(false, true, false),

            (Menu(false, false, true), A) => Menu(false, true, false),
            (Menu(false, false, true), Continue) => Menu(false, false, true),
            // down
            (Menu(true, false, false), D) => Menu(false, true, false),

            (Menu(false, true, false), D) => Menu(false, false, true),
            (Menu(false, false, true), D) => Menu(true, false, false),

            (Menu(false, false, false), Continue) => Menu(false, false, false),
            (Menu(false, false, false), A) => Menu(true, false, false),
            (Menu(false, false, false), D) => Menu(false, false, true),
            (_, _) => DisplayTime,
        }
    }
}
// TODO(elsuizo: 2024-08-07): ver como se puede hacer para serialize el `DateTime`
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ClockFromPc<'a> {
    time: &'a str,
}

pub struct Clock<'r, T: Instance> {
    rtc: Rtc<'r, T>,
    alarm: Option<DateTime>,
    periodic: bool,
}

impl<'r, T: Instance + 'r> Clock<'r, T> {
    pub fn new(user_time_set: DateTime, mut rtc: Rtc<'static, T>) -> Result<Self, RtcError> {
        rtc.set_datetime(user_time_set)?;
        Ok(Self {
            rtc,
            alarm: None,
            periodic: false,
        })
    }

    pub fn read(&self) -> String<256> {
        let mut time: String<256> = String::new();
        if let Ok(dt) = self.rtc.now() {
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

    pub fn set_alarm(&mut self, alarms: DateTimeFilter) {
        self.rtc.schedule_alarm(alarms);
    }
}
