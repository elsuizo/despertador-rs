use defmt::Format;
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
