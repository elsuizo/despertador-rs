use crate::ui::Msg;
use crate::String;
use core::fmt::Write;
use defmt::info;
use embassy_rp::peripherals::RTC;
use embassy_rp::rtc::{DateTime, DateTimeFilter, DayOfWeek, Instance, Rtc, RtcError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum ClockState {
    Time,
    Alarm,
    Image,
}

// #[derive(Copy, Clone)]
// pub enum Msg {
//     Up,       // Up button
//     Down,     // Down button
//     Continue, // Continue in the actual state
// }

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
            (Time, A) => Alarm,
            (Time, D) => Image,
            (Time, Continue) => Time,
            (Time, _) => Time,
            (Alarm, A) => Image,
            (Alarm, D) => Time,
            (Alarm, Continue) => Alarm,
            (Alarm, _) => Alarm,
            (Image, A) => Alarm,
            (Image, D) => Time,
            (Image, Continue) => Image,
            (Image, _) => Image,
        }
    }
}
// TODO(elsuizo: 2024-08-07): ver como se puede hacer para serialize el `DateTime`
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ClockFromPc<'a> {
    time: &'a str,
}

pub struct Clock<'r, T: Instance> {
    now: DateTime,
    rtc: Rtc<'r, T>,
    alarm: Option<DateTime>,
    periodic: bool,
}

impl<'r, T: Instance + 'r> Clock<'r, T> {
    pub fn new(actual_time: DateTime, mut rtc: Rtc<'static, T>) -> Result<Self, RtcError> {
        let now = rtc.now()?;
        rtc.set_datetime(actual_time)?;
        Ok(Self {
            now,
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
