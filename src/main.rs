use sdl2;

const MEM_SIZE: usize = 4096;
const DISP_W: usize = 64;
const DISP_H: usize = 32;
const SCALE: usize = 10;
const WIN_W: usize = DISP_W * SCALE;
const WIN_H: usize = DISP_H * SCALE;
const FPS: usize = 60;
const CYCLES: usize = 700 / FPS;

pub struct Chip8 {
    pub mem: [u8; MEM_SIZE],
    pub disp: [u8; DISP_W * DISP_H],
    pub v: [u8; 16],
    pub i: u16,
    pub pc: u16,
    pub stack: [u16; 16],
    pub sp: u8,
    pub keys: [bool; 16],
    pub delay: u8,
    pub sound: u8,
    pub sound_active: bool,
    pub window: sdl2::video::Window,
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub last_cycle_time: u64,
    pub debug: bool,
}

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0, 0xF0,
    0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0, 0xF0, 0x80,
    0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0,
    0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80,
    0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
];

fn opcode_str(op: u16) -> String {
    const REGS: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
    ];

    let (x, y, n, kk) = (
        ((op >> 8) & 0xF) as usize,
        (op >> 4) & 0xF,
        op & 0xF,
        op & 0xFF,
    );

    let vx = REGS[x];
    let vy = REGS[y as usize];

    let nnn = op & 0xFFF;

    match op & 0xF000 {
        0x0000 => match op {
            0x00E0 => "CLS".to_string(),
            0x00EE => "RET".to_string(),
            _ => format!("SYS 0x{:03X}", nnn),
        },
        0x1000 => format!("JP 0x{:03X}", nnn),
        0x2000 => format!("CALL 0x{:03X}", nnn),
        0x3000 => format!("SE V{}, 0x{:02X}", vx, kk),
        0x4000 => format!("SNE V{}, 0x{:02X}", vx, kk),
        0x5000 => format!("SE V{}, V{}", vx, vy),
        0x6000 => format!("LD V{}, 0x{:02X}", vx, kk),
        0x7000 => format!("ADD V{}, 0x{:02X}", vx, kk),
    }
}

fn main() {}
