use rand::SeedableRng;
mod emu;
mod screen;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    let interpreter = std::fs::read("interpreter.bin")?;
    let game = std::fs::read("test_opcode.ch8")?;
    let rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut c8 = emu::Chip8::new(&interpreter, &game, Box::new(rng) as _);
    drop(interpreter);
    drop(game);
    let mut s = screen::Screen::new();

    while running.load(Ordering::SeqCst) {
        c8.step();
        s.draw(&c8.screen);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}
