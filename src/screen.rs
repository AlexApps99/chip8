use std::io::Write;

pub struct Screen {}

impl Screen {
    pub fn new() -> Self {
        // Hide cursor
        print!("\x1b[?25l");
        Self::flush();
        Self {}
    }

    fn flush() {
        let _ = std::io::stdout().flush();
    }

    fn clear() {
        //print!("\x1b[2J\x1b[3J\x1b[1;1H");
        Self::flush();
    }

    pub fn draw(&mut self, bits: &[[u8; 64]; 32]) {
        Self::clear();
        // for col in 0..32 {
        //   for byte in bits[8*col..8*col+8].iter() {
        //     for x in 0..8 {
        //       if ((byte >> x) & 1) != 0 { print!("\u{2588}") } else { print!(" ") }
        //     }
        //   }
        //   println!();
        //   Self::flush();
        // }
        for row in bits {
            for pixel in row {
                if *pixel != 0 {
                    print!("\u{2588}")
                } else {
                    print!(" ")
                }
            }
            println!();
            Self::flush();
        }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        // Show cursor
        print!("\x1b[?25h");
        Self::clear();
    }
}
