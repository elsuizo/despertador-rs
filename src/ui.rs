//----------------------------------------------------------------------------
// @file clock.rs
//
// @date 2024-08-08
// @author Martin Noblia
// @email mnoblia@disroot.org
//
// @brief
//
// @detail
//
// Licence MIT:
// Copyright filename <Martin Noblia>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.  THE SOFTWARE IS PROVIDED
// "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
// LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
// PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
// HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//----------------------------------------------------------------------------
use defmt::Format;
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_10X20, ascii::FONT_9X15, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

#[derive(Copy, Clone, Debug, Format)]
pub enum Msg {
    A,        // A keypad Button
    B,        // B keypad Button
    C,        // C keypad Button
    D,        // D keypad Button
    One,      // 1 keypad Button
    Two,      // 2 keypad Button
    Three,    // 3 keypad Button
    Four,     // 4 keypad Button
    Five,     // 5 keypad Button
    Six,      // 6 keypad Button
    Seven,    // 7 keypad Button
    Eight,    // 8 keypad Button
    Nine,     // 9 keypad Button
    Asterisk, // * keypad Button
    Zero,     // 0 keypad Button
    Numeral,  // # keypad Button
    Continue, // Continue in the actual state
}

pub fn show_menu<D>(target: &mut D, state: (bool, bool, bool)) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // normal text
    let normal = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();
    // text with background
    let background = MonoTextStyleBuilder::from(&normal)
        .background_color(BinaryColor::On)
        .text_color(BinaryColor::Off)
        .build();

    match state {
        (true, false, false) => {
            let _ = Text::new("Set Time", Point::new(10, 13), background).draw(target);
            let _ = Text::new("Set Alarm", Point::new(10, 13 + 20), normal).draw(target);
            let _ = Text::new("Test sound", Point::new(10, 13 + 20 + 20), normal).draw(target);
        }
        (false, true, false) => {
            let _ = Text::new("Set Time", Point::new(10, 13), normal).draw(target);
            let _ = Text::new("Set Alarm", Point::new(10, 13 + 20), background).draw(target);
            let _ = Text::new("Test sound", Point::new(10, 13 + 20 + 20), normal).draw(target);
        }
        (false, false, true) => {
            let _ = Text::new("Set Time", Point::new(10, 13), normal).draw(target);
            let _ = Text::new("Set Alarm", Point::new(10, 13 + 20), normal).draw(target);
            let _ = Text::new("Test sound", Point::new(10, 13 + 20 + 20), background).draw(target);
        }
        (false, false, false) => {
            let _ = Text::new("Set Time", Point::new(10, 13), normal).draw(target);
            let _ = Text::new("Set Alarm", Point::new(10, 13 + 20), normal).draw(target);
            let _ = Text::new("Test sound", Point::new(10, 13 + 20 + 20), normal).draw(target);
        }
        (_, _, _) => panic!("invalid display state"),
    }
    Ok(())
}
