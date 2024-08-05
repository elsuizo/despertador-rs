use crate::String;
use core::fmt::Write;
use defmt::info;
use embassy_rp::peripherals::RTC;
use embassy_rp::rtc::{DateTime, DayOfWeek, Instance, Rtc};

// TODO(elsuizo: 2024-08-04): this should be `Serializable` with the postcard crate???
pub struct Alarm<'r, T: Instance> {
    rtc: Rtc<'r, T>,
    alarm: Option<DateTime>,
    periodic: bool,
}

impl<'r, T: Instance + 'r> Alarm<'r, T> {
    pub fn new(actual_time: DateTime, mut rtc: Rtc<'static, T>) -> Self {
        rtc.set_datetime(actual_time).unwrap();
        Self {
            rtc,
            alarm: None,
            periodic: false,
        }
    }

    pub fn clock_read(&self) -> String<256> {
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
}
