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

    pub fn draw(&mut self, bits: &[u64; 32]) {
        Self::clear();
        for row in bits {
            for x in 0..64 {
                if ((row >> (63 - x)) & 1) != 0 {
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
