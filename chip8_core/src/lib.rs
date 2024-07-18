// 64x32 monochrome display (1 bit per pixel)
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096; // 4kb
const NUM_REGS: usize = 16; // # of V register
const STACK_SIZE: usize = 16; // Stack Pointer (SP)
const NUM_KEYS: usize = 16;
const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80 // F
];

pub struct Emu {
    // Program Counter (PC) - special register that stores index of current instruction
    pc: u16,
    // 4,096 bytes of ram
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    // 16 V registers (V0-VF)
    v_reg: [u8; NUM_REGS],
    // I register
    i_reg: u16, 
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    // Delay Timer
    dt: u8,
    // Sound Timer
    st: u8,

}

const START_ADDR: u16 = 0x200; // 512

impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };

        // copy_from_slice: Copies all elemenmts from src into self
        // [..FONTSET_SIZE] means that we're essentially doing [0..80] then copying all the values
        // of FONTSET into the 0..80 indices
        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        new_emu
    }

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        // Fetch
        let op = self.fetch();
        self.execute(op);
    }

    fn execute(&mut self, op: u16) {
        // (from left to right)
        // we mask the nibble at the position we want then shift all bits over all bits by a number
        // that will place that nibble in the first four positions
        // the last digit doesn't need a shift since it's already in the desired position
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // 0000 - NOP - Nop
            (0, 0, 0, 0) => return,
            // 00E0 - CLS - Clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            },
            // 00EE - RET - Return from Subroutine
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
            (0,0,0xE,0xE) => {
                let ret_addr = self.pop();
                self.pc = ret_addr;
            },
            // 1NNN - JMP NNN - Jump
            (1, _,_,_) => {
                // 0xFFF gets us the lower 12 bits
                // e.g.
                /*    
                      1010101111001101 (0xABCD)
                   &  0000111111111111 (0xFFF)
                      ----------------------
                      0000101111001101 (0x0BCD)
                */
                let nnn = op & 0xFFF;
                self.pc = nnn;
            },
            // 2NNN - CALL NNN - Call Subroutine
            (2,_,_,_) => {
                let nnn = op & 0xFFF;
                self.push(self.pc);
                self.pc = nnn;
            },
            // 3XNN - SKIP VX == NN - Skip next if VX == NN
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    self.pc += 2;
                }
            },
            // 4XNN - SKIP VX != NN - Skip next if VX != NN
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    self.pc += 2;
                }
            },
            // 5XY0 - SKIP VX == VY - Skip next if VX == VY
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2;
                }
            },
            // 6XNN - VX = NN
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = nn;
            },
            // 7XNN - VX += NN
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn); // wrapping_add for possible overflow
            },
            // 8XY0 - VX = VY
            (8,_,_,0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            },
            // 8XY1, 8XY2, 8XY3 - Bitwise operations
            // 8XY1 - VX != VY
            (8,_,_,1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] != self.v_reg[y];
            },
            // 8XY2 - VX &= VY
            (8,_,_,2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            },
            // 8XY3 - VX ^= VY
            (8,_,_,3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            },
            // 8XY4 - VX += VY
            (8,_,_,4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
                let new_vf = if carry { 1 } else { 0 };
                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }
        }
    }
    
    // not public since only called internally
    fn fetch (&mut self) -> u16 {
        // fetch these two parts as u16 to enable shifting to left of hbyte and then bitwise OR the
        // lbyte into where the hbyte used to be
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        // shift higher_byte 8 bits to the left
        let op = (higher_byte << 8) | lower_byte; 
        self.pc += 2;
        op
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 { 
            self.dt -= 1;
        }

        if self.st > 0 {
            if self.st == 1 {
                // BEEP
            }
            self.st -= 1;
        }
    }
}
