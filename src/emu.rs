use std::convert::TryInto;

pub struct Chip8 {
    wrap_tex: bool,
    hp_shift: bool,
    mem_inc: bool,
    // Time of last decrement (timers)
    last_dec: std::time::Instant,
    // RNG
    rng: Box<dyn rand::RngCore>,
    // V registers
    v: [u8; 16], // Possibly provide more registers than vanilla
    // I register
    i: u16,
    // Delay timer
    dt: u8,
    // Sound timer
    st: u8,
    // Program counter
    pc: u16,
    // Stack pointer
    sp: u8,
    // Stack
    stk: [u16; 16], // Bigger stack?
    // RAM
    ram: [u8; 4096],
    pub screen: [u64; 32],
}

// Memory address (12 bits)
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
struct Addr(pub u16);

// V register (4 bits)
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
struct VReg(pub u8);

#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
enum Instruction {
    /// Clear screen
    CLS,
    /// Return from subroutine
    RET,
    /// Jump
    JP(Addr),
    /// Call subroutine
    CALL(Addr),
    /// Skip next if Vx == u8
    SEB(VReg, u8),
    /// Skip next if Vx != u8
    SNEB(VReg, u8),
    /// Skip next if Vx == Vy
    SEV(VReg, VReg),
    /// Vx = u8
    LDB(VReg, u8),
    /// Vx = Vx + u8
    ADDB(VReg, u8),
    /// Vx = Vy
    LDV(VReg, VReg),
    /// Vx = Vx | Vy
    OR(VReg, VReg),
    /// Vx = Vx & Vy
    AND(VReg, VReg),
    /// Vx = Vx ^ Vy
    XOR(VReg, VReg),
    /// Vx = Vx + Vy, VF = carry
    ADDC(VReg, VReg),
    /// Vx = Vx - Vy, VF = NOT borrow
    SUB(VReg, VReg),
    ///  Vx = Vx >> 1
    SHR(VReg, VReg),
    /// Vx = Vy - Vx, VF = NOT borrow
    SUBN(VReg, VReg),
    /// Vx = Vx << 1
    SHL(VReg, VReg),
    /// Skip next if Vx != Vy
    SNEV(VReg, VReg),
    /// I = Addr
    LDI(Addr),
    /// Jump to Addr + V0
    JPV(Addr),
    /// Vx = random & u8
    RND(VReg, u8),
    /// Display (nibble)
    DRW(VReg, VReg, u8),
    /// Skip next if key Vx is pressed
    SKP(VReg),
    /// Skip next if key Vx is not pressed
    SKNP(VReg),
    /// Vx = DT
    LDVD(VReg),
    /// Wait for key, Vx = key
    LDK(VReg),
    /// DT = Vx
    LDDV(VReg),
    /// ST = Vx
    LDSV(VReg),
    /// I = I + Vx
    ADDI(VReg),
    /// I = location of sprite for digit Vx
    LDIS(VReg),
    /// Store BCD of Vx in I, I+1, I+2
    LDD(VReg),
    /// Store V0 to Vx in memory starting at I
    LDMV(VReg),
    /// Read memory starting at I into V0 to Vx
    LDVM(VReg),
}

impl Instruction {
    pub fn decode(ins: &u16) -> Option<Self> {
        use Instruction::*;
        match ins {
            0x00E0 => return Some(Self::CLS),
            0x00EE => return Some(Self::RET),
            _ => (),
        }
        let i = ins & 0xF000;
        let i2 = ins & 0xF00F;
        let i3 = ins & 0xF0FF;
        let addr = Addr(ins & 0x0FFF);
        let x = VReg(((ins & 0x0F00) >> 8u16) as u8);
        let y = VReg(((ins & 0x00F0) >> 4u16) as u8);
        let kk = (ins & 0x00FF) as u8;
        match i {
            // SYS
            0x0000 => None,
            0x1000 => Some(JP(addr)),
            0x2000 => Some(CALL(addr)),
            0x3000 => Some(SEB(x, kk)),
            0x4000 => Some(SNEB(x, kk)),
            0x6000 => Some(LDB(x, kk)),
            0x7000 => Some(ADDB(x, kk)),
            0xA000 => Some(LDI(addr)),
            0xB000 => Some(JPV(addr)),
            0xC000 => Some(RND(x, kk)),
            0xD000 => Some(DRW(x, y, (ins & 0x000F) as u8)),
            _ => match i2 {
                0x5000 => Some(SEV(x, y)),
                0x8000 => Some(LDV(x, y)),
                0x8001 => Some(OR(x, y)),
                0x8002 => Some(AND(x, y)),
                0x8003 => Some(XOR(x, y)),
                0x8004 => Some(ADDC(x, y)),
                0x8005 => Some(SUB(x, y)),
                0x8006 => Some(SHR(x, y)),
                0x8007 => Some(SUBN(x, y)),
                0x800E => Some(SHL(x, y)),
                0x9000 => Some(SNEV(x, y)),
                _ => match i3 {
                    0xE09E => Some(SKP(x)),
                    0xE0A1 => Some(SKNP(x)),
                    0xF007 => Some(LDVD(x)),
                    0xF00A => Some(LDK(x)),
                    0xF015 => Some(LDDV(x)),
                    0xF018 => Some(LDSV(x)),
                    0xF01E => Some(ADDI(x)),
                    0xF029 => Some(LDIS(x)),
                    0xF033 => Some(LDD(x)),
                    0xF055 => Some(LDMV(x)),
                    0xF065 => Some(LDVM(x)),
                    _ => None,
                },
            },
        }
    }

    // TODO make invalid V registers a noop
    // TODO Use common sense to avoid crashing and undefined/weird behavior
    // TODO Make sure there are no boundary cases where it is/isn't allowed and shouldn't be
    pub fn execute(&self, c8: &mut Chip8) {
        use Instruction::*;
        match self {
            CLS => c8.screen = [0u64; 32],
            RET => {
                let sp = c8.sp as usize;
                if sp > 0 {
                    c8.pc = c8.stk[sp];
                    c8.stk[sp] = 0;
                    c8.sp -= 1;
                }
            }
            JP(addr) => {
                if (addr.0 as usize) < c8.ram.len() {
                    c8.pc = addr.0
                }
            }
            CALL(addr) => {
                let sp = c8.sp as usize;
                if sp < c8.stk.len() {
                    c8.stk[sp] = c8.pc;
                    c8.pc = addr.0;
                    c8.sp += 1;
                }
            }
            SEB(x, kk) => {
                if *c8.get_v(x) == *kk {
                    c8.pc += 2
                }
            }
            SNEB(x, kk) => {
                if *c8.get_v(x) != *kk {
                    c8.pc += 2
                }
            }
            SEV(x, y) => {
                if *c8.get_v(x) == *c8.get_v(y) {
                    c8.pc += 2
                }
            }
            LDB(x, kk) => *c8.get_v(x) = *kk,
            ADDB(x, kk) => {
                let (v, _) = c8.get_v(x).overflowing_add(*kk);
                *c8.get_v(x) = v;
            }
            LDV(x, y) => *c8.get_v(x) = *c8.get_v(y),
            OR(x, y) => *c8.get_v(x) |= *c8.get_v(y),
            AND(x, y) => *c8.get_v(x) &= *c8.get_v(y),
            XOR(x, y) => *c8.get_v(x) ^= *c8.get_v(y),
            ADDC(x, y) => {
                let (v, flag) = c8.get_v(x).overflowing_add(*c8.get_v(y));
                *c8.get_v(x) = v;
                c8.v[15] = if flag { 1 } else { 0 };
            }
            SUB(x, y) => {
                let (v, flag) = c8.get_v(x).overflowing_sub(*c8.get_v(y));
                *c8.get_v(x) = v;
                c8.v[15] = if flag { 0 } else { 1 };
            }
            SHR(x, y) => {
                if c8.hp_shift {
                    c8.v[15] = *c8.get_v(x) & 1;
                    *c8.get_v(x) = *c8.get_v(x) >> 1;
                } else {
                    c8.v[15] = *c8.get_v(y) & 1;
                    *c8.get_v(x) = *c8.get_v(y) >> 1;
                }
            }
            SUBN(x, y) => {
                let (v, flag) = c8.get_v(y).overflowing_sub(*c8.get_v(x));
                *c8.get_v(x) = v;
                c8.v[15] = if flag { 0 } else { 1 };
            }
            SHL(x, y) => {
                if c8.hp_shift {
                    c8.v[15] = *c8.get_v(x) >> 7;
                    *c8.get_v(x) = *c8.get_v(x) << 1;
                } else {
                    c8.v[15] = *c8.get_v(y) >> 7;
                    *c8.get_v(x) = *c8.get_v(y) << 1;
                }
            }
            SNEV(x, y) => {
                if *c8.get_v(x) != *c8.get_v(y) {
                    c8.pc += 2
                }
            }
            LDI(addr) => {
                if (addr.0 as usize) < c8.ram.len() {
                    c8.i = addr.0
                }
            }
            JPV(addr) => c8.pc = addr.0 + c8.v[0] as u16,
            RND(x, kk) => {
                let mut val = [0u8; 1];
                c8.rng.fill_bytes(&mut val);
                *c8.get_v(x) = val[0] & kk;
            }
            DRW(x, y, n) => {
                let i = c8.i as usize;
                let sz = *n as usize;
                if sz > 0 && i + sz <= c8.ram.len() {
                    let (px, py) = if c8.wrap_tex {
                        (*c8.get_v(x) as usize, *c8.get_v(y) as usize)
                    } else {
                        (
                            ((*c8.get_v(x)) % 64) as usize,
                            ((*c8.get_v(y)) % 32) as usize,
                        )
                    };
                    c8.v[15] = 0;
                    for r in 0..sz {
                        let sprite = c8.ram[i + r];
                        let cy = py + r;
                        if cy < c8.screen.len() {
                            c8.screen[cy] ^= if px > 56 {
                                (sprite as u64) >> (px - 56)
                            } else {
                                (sprite as u64) << (56 - px)
                            };
                        }
                    }
                }
            }
            SKP(_) => {}           // TODO
            SKNP(_) => c8.pc += 2, // TODO
            LDVD(x) => *c8.get_v(x) = c8.dt,
            LDK(_) => unimplemented!(),
            LDDV(x) => c8.dt = *c8.get_v(x),
            LDSV(x) => c8.st = *c8.get_v(x),
            ADDI(x) => c8.i += *c8.get_v(x) as u16,
            LDIS(x) => c8.i = 0x050 + (*c8.get_v(x) * 5) as u16,
            LDD(x) => {
                let i = c8.i as usize;
                if i + 2 < c8.ram.len() {
                    let num = *c8.get_v(x);
                    c8.ram[i] = num / 100;
                    c8.ram[i + 1] = (num / 10) % 10;
                    c8.ram[i + 2] = num % 10;
                }
            }
            LDMV(x) => {
                let i = c8.i as usize;
                let space = x.0 as usize;
                if i + space < c8.ram.len() && space < c8.v.len() {
                    c8.ram[i..=i + space].copy_from_slice(&c8.v[0..=space]);
                    if c8.mem_inc {
                        c8.i += x.0 as u16 + 1;
                    }
                }
            }
            LDVM(x) => {
                let i = c8.i as usize;
                let space = x.0 as usize;
                if i + space < c8.ram.len() && space < c8.v.len() {
                    c8.v[0..=space].copy_from_slice(&c8.ram[i..=i + space]);
                    if c8.mem_inc {
                        c8.i += x.0 as u16 + 1;
                    }
                }
            }
        }
    }
}

impl Chip8 {
    pub fn new(int: &[u8], rom: &[u8], rng: Box<dyn rand::RngCore>) -> Self {
        let mut ram: Vec<u8> = Vec::with_capacity(4096);
        ram.extend_from_slice(&int);
        ram.resize(512, 0);
        ram.extend_from_slice(&rom);
        ram.resize(4096, 0);

        Self {
            wrap_tex: true,
            hp_shift: true,
            mem_inc: true,
            last_dec: std::time::Instant::now(),
            rng: Box::new(rng),
            v: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            pc: 0x200,
            sp: 0,
            stk: [0; 16],
            ram: ram.try_into().unwrap(),
            screen: [0u64; 32],
        }
    }

    // Clamp v to max value so no out of range access
    pub(self) fn get_v(&mut self, n: &VReg) -> &mut u8 {
        let l = n.0 as usize;
        &mut self.v[if l < self.v.len() {
            l
        } else {
            self.v.len() - 1
        }]
    }

    pub fn step(&mut self) {
        let now = std::time::Instant::now();
        let steps = (now.duration_since(self.last_dec).as_secs_f64() * 60.0).max(255.0) as u8;
        if steps > 0 {
            self.last_dec = now;
            self.dt = if steps > self.dt { 0 } else { self.dt - steps };
            self.st = if steps > self.st { 0 } else { self.st - steps }
        }
        let idx = self.pc as usize;
        if idx + 1 < self.ram.len() {
            let val: u16 = ((self.ram[idx] as u16) << 8) | self.ram[idx + 1] as u16;
            let ins = Instruction::decode(&val);
            println!("{:?}", ins);
            if let Some(i) = ins {
                i.execute(self);
            }
            self.pc += 2;
        }
    }
}
