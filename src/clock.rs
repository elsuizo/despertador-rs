use crate::{ui::Msg, ALARM_ENABLED};
use defmt::info;
use embassy_rp::rtc::{DateTime, DateTimeFilter, DayOfWeek, Instance, Rtc, RtcError};
// TODO(elsuizo: 2026-05-12): esto es para cuando hagamos lo de la conexion UART
//use serde::{Deserialize, Serialize};
use crate::TIME_CHANNEL;

#[derive(Debug, Clone)]
pub enum ClockState {
    DisplayTime(DateTime),
    SetTime(DateTime),
    DisplayAlarm,
    SetAlarm(bool),
    ShowImage,
    TestSound,
    StopAlarm,
    Alarm,
    Menu(bool, bool, bool), // menu rows
}

#[derive(Clone, Debug)]
pub struct ClockFSM {
    pub state: ClockState,
    pub now: DateTime,
}

impl ClockFSM {
    pub fn init(state: ClockState, now: DateTime) -> Self {
        Self { state, now }
    }

    pub async fn next_state(&mut self, msg: Msg) {
        use ClockState::*;
        use Msg::*;

        self.state = match (self.state.clone(), msg) {
            (StopAlarm, _) => DisplayTime(self.now.clone()),
            (DisplayTime(_), A) => DisplayAlarm,
            (DisplayTime(date), C) => SetTime(date),
            (DisplayTime(date), Continue) => DisplayTime(date),
            (DisplayTime(_), Numeral) => ShowImage,
            (ShowImage, Continue) => ShowImage,
            (ShowImage, _) => DisplayTime(self.now.clone()),
            (DisplayTime(_), B) => Menu(false, false, false),
            (DisplayTime(_), AlarmEvent) => Alarm,
            (DisplayTime(date), _) => DisplayTime(date), // any other keys
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
            // TestSound trigger
            (Menu(false, false, true), Asterisk) => TestSound,
            (TestSound, Continue) => TestSound,
            (TestSound, A) => DisplayTime(self.now.clone()),
            (TestSound, _) => TestSound,
            // SetTime trigger
            (Menu(true, false, false), Asterisk) => SetTime(self.now.clone()),
            (SetTime(date_time), Continue) => SetTime(date_time),
            (SetTime(ref mut date @ DateTime { year: y, .. }), Numeral) => {
                date.year = if y + 1 < 4095 { y + 1 } else { 0 };
                SetTime(date.clone())
            }
            (SetTime(ref mut date @ DateTime { month: m, .. }), One) => {
                date.month = if m + 1 < 13 { m + 1 } else { 0 };
                SetTime(date.clone())
            }
            (SetTime(ref mut date @ DateTime { day: d, .. }), Two) => {
                date.day = if d + 1 < 32 { d + 1 } else { 0 };
                SetTime(date.clone())
            }
            // hour + 1
            (SetTime(ref mut date @ DateTime { hour: h, .. }), A) => {
                date.hour = if h + 1 < 24 { h + 1 } else { 0 };
                SetTime(date.clone())
            }
            (SetTime(ref mut date @ DateTime { minute: m, .. }), B) => {
                date.minute = if m + 1 < 60 { m + 1 } else { 0 };
                SetTime(date.clone())
            }
            (SetTime(ref mut date @ DateTime { second: s, .. }), C) => {
                date.second = if s + 1 < 60 { s + 1 } else { 0 };
                SetTime(date.clone())
            }
            (
                SetTime(
                    ref mut date @ DateTime {
                        day_of_week: day, ..
                    },
                ),
                D,
            ) => {
                date.day_of_week = if day as u8 + 1 < 7 {
                    day_of_week_from_u8(day as u8 + 1)
                } else {
                    DayOfWeek::Sunday
                };
                SetTime(date.clone())
            }
            // TODO(elsuizo: 2026-05-20): clones everywere!!!
            (SetTime(date), Zero) => {
                TIME_CHANNEL.sender().send(date.clone()).await;
                DisplayTime(date.clone())
            }

            // SetAlarm trigger
            (Menu(false, true, false), Asterisk) => SetAlarm(true),
            (SetAlarm(state), Continue) => SetAlarm(state),
            // disable alarm
            (SetAlarm(true), Asterisk) => SetAlarm(false),
            // enable alarm again
            (SetAlarm(false), Asterisk) => SetAlarm(true),

            (Menu(false, false, false), Continue) => Menu(false, false, false),
            (Menu(false, false, false), A) => Menu(true, false, false),
            (Menu(false, false, false), D) => Menu(false, false, true),
            // (StopAlarm, _) => DisplayTime,
            (DisplayAlarm, Continue) => DisplayAlarm,
            (DisplayAlarm, _) => DisplayTime(self.now.clone()),
            (Alarm, Continue) => Alarm,
            (Alarm, Zero) => {
                info!("Alarm stopped");
                StopAlarm
            }
            (_, _) => DisplayTime(self.now.clone()),
        }
    }
}

// todo(elsuizo: 2024-08-07): ver como se puede hacer para serialize el `DateTime`
//#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
//pub struct ClockFromPc<'a> {
//    time: &'a [u8],
//}

fn day_of_week_from_u8(v: u8) -> DayOfWeek {
    match v {
        0 => DayOfWeek::Sunday,
        1 => DayOfWeek::Monday,
        2 => DayOfWeek::Tuesday,
        3 => DayOfWeek::Wednesday,
        4 => DayOfWeek::Thursday,
        5 => DayOfWeek::Friday,
        6 => DayOfWeek::Saturday,
        _ => panic!("error that day not exists"),
    }
}

pub struct Clock<'r, T: Instance> {
    pub rtc: Rtc<'r, T>,
    alarm: Option<DateTimeFilter>,
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

    pub fn read(&self) -> DateTime {
        self.rtc.now().expect("Error reading the clock state")
    }

    pub fn set_alarm(&mut self, alarm: DateTimeFilter) {
        self.alarm = Some(alarm);
        self.rtc.schedule_alarm(alarm);
    }

    pub fn disable_alarm(&mut self) {
        self.rtc.disable_alarm();
        self.alarm = None;
    }

    pub fn alarm_is_enabled(&self) -> bool {
        self.alarm.is_some()
    }

    // TODO(elsuizo: 2026-05-14): maybe here could add a parameter for the period time
    pub fn enable_periodic_alarm(&mut self) {
        self.periodic = true
    }

    pub fn disable_periodic_alarm(&mut self) {
        self.periodic = false
    }

    pub fn alarm_is_periodic(&self) -> bool {
        self.periodic
    }

    pub async fn wait_alarm(&mut self) {
        self.rtc.wait_for_alarm().await;
    }
}
