use crate::String;
use defmt::info;
//-------------------------------------------------------------------------
//                        clock main task
//-------------------------------------------------------------------------
#[embassy_executor::task]
pub async fn clock_update(
    mut button_command_input: ButtonMessageSub,
    i2c: embassy_rp::i2c::I2c<'static, I2C1, embassy_rp::i2c::Blocking>,
    rtc: embassy_rp::rtc::Rtc<'static, RTC>,
) {
    let mut ticker = Ticker::every(Duration::from_millis(1000));
    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();
    display.init().ok();
    display.flush().ok();
    let mut counter = 0;

    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();

    loop {
        let time = clock_read(&rtc);
        display.flush().ok();
        display.clear();
        info!("clock update");
        Text::new(&time, Point::new(37, 13), normal).draw(&mut display);
        ticker.next().await;
    }
}

fn clock_read<'r, T: Instance + 'r>(rtc: &Rtc<'r, T>) -> String<256> {
    let mut time: String<256> = String::new();
    if let Ok(dt) = rtc.now() {
        write!(
            &mut time,
            "{:02}:{:02}:{:02}",
            dt.hour, dt.minute, dt.second,
        )
        .unwrap();
    } else {
        info!("The RTC is not working ...")
    }
    time
}
