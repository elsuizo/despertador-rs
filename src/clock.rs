use crate::String;
use core::fmt::Write;
use defmt::info;
use embassy_rp::rtc::{DateTime, DayOfWeek, Instance, Rtc};

// TODO(elsuizo: 2024-08-04): this should be `Serializable` with the postcard crate???
pub struct Alarm {
    actual_time: DateTime,
    alarm: Option<DateTime>,
    periodic: bool,
}

impl Alarm {
    pub fn new(actual_time: DateTime) -> Self {
        Self {
            actual_time,
            alarm: None,
            periodic: false,
        }
    }

    pub fn clock_read<'r, T: Instance + 'r>(&self, rtc: &Rtc<'r, T>) -> String<256> {
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
}
