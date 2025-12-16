use rand::Rng;
use sdl2;
use sdl2::keyboard::Scancode;
use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

const MEM_SIZE: usize = 4096;
const DISP_W: usize = 64;
const DISP_H: usize = 32;
const SCALE: usize = 10;
const WIN_W: u32 = (DISP_W * SCALE) as u32;
const WIN_H: u32 = (DISP_H * SCALE) as u32;
const FPS: usize = 60;
const CYCLES: usize = 700 / FPS;

pub struct Chip8 {
    pub mem: [u8; MEM_SIZE as usize],
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
        0x8000 => match n {
            0x0 => format!("LD V{}, V{}", vx, vy),
            0x1 => format!("OR  V{}, V{}", vx, vy),
            0x2 => format!("AND V{}, V{}", vx, vy),
            0x3 => format!("XOR V{}, V{}", vx, vy),
            0x4 => format!("ADD V{}, V{}", vx, vy),
            0x5 => format!("SUB V{}, V{}", vx, vy),
            0x6 => format!("SHR V{}", vx),
            0x7 => format!("SUBN V{}, V{}", vx, vy),
            0xE => format!("SHL V{}", vx),
            _ => format!("8XY{} ??", n),
        },
        0x9000 => format!("SNE V{}, V{}", vx, vy),
        0xA000 => format!("LD I, 0x{:03X}", nnn),
        0xB000 => format!("JP V0, 0x{:03X}", nnn),
        0xC000 => format!("RND V{}, 0x{:02X}", vx, kk),
        0xD000 => format!("DRW V{}, V{}, {}", vx, vy, n),
        0xE000 => {
            if kk == 0x9E {
                format!("SKP V{}", vx)
            } else if kk == 0xA1 {
                format!("SKNP V{}", vx)
            } else {
                format!("EX{} ??", kk)
            }
        }
        0xF000 => match kk {
            0x07 => format!("LD V{}, DT", vx),
            0x0A => format!("LD V{}, K", vx),
            0x15 => format!("LD DT, V{}", vx),
            0x18 => format!("LD ST, V{}", vx),
            0x1E => format!("ADD I, V{}", vx),
            0x29 => format!("LD F, V{}", vx),
            0x33 => format!("LD BCD, V{}", vx),
            0x55 => format!("LD [I], V{}", vx),
            0x65 => format!("LD V{}, [I]", vx),
            _ => format!("FX{:02X} ??", kk),
        },
        _ => format!("0x{:04X} ??", op),
    }
}

impl Chip8 {
    pub fn new(debug: bool) -> Result<Self, String> {
        let sdl_init = sdl2::init()?;
        let video_subsystem = sdl_init.video()?;

        let window = video_subsystem
            .window("CHIP8", WIN_W, WIN_H)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .clone()
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        canvas
            .set_logical_size(DISP_W as u32, DISP_H as u32)
            .map_err(|e| e.to_string())?;

        let mut c = Chip8 {
            mem: [0; MEM_SIZE as usize],
            disp: [0_u8; DISP_W * DISP_H],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            keys: [false; 16],
            delay: 0,
            sound: 0,
            sound_active: false,
            window: window,
            canvas: canvas,
            last_cycle_time: 0,
            debug,
        };

        c.mem[0..FONT.len()].copy_from_slice(&FONT);
        Ok(c)
    }
    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let mut f = File::open(path).map_err(|e| format!("failed to open rom:{e}"))?;
        let metadata = f.metadata().map_err(|e| e.to_string())?;
        let sz = metadata.len() as usize;

        const PROGRAM_START: usize = 0x200;
        let max_size = MEM_SIZE - PROGRAM_START;

        if sz > max_size {
            return Err(format!(
                "ROM size ({}) exceed maximum allowed size ({})",
                sz, max_size,
            ));
        }
        f.read_exact(&mut self.mem[PROGRAM_START..PROGRAM_START + sz])
            .map_err(|e| format!("failed to read ROM dataa:{e}"))?;

        Ok(())
    }
    pub fn clear_d(&mut self) {
        self.disp.fill(0);
    }
    pub fn draw(&mut self) -> Result<(), String> {
        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));

        for i in 0..self.disp.len() {
            if self.disp[i] != 0 {
                let x = i % DISP_W;
                let y = i / DISP_W;

                self.canvas
                    .draw_point(sdl2::rect::Point::new(x as i32, y as i32))
                    .map_err(|e| e.to_string())?;
            }
        }
        self.canvas.present();
        Ok(())
    }
    pub fn input(&mut self, k: &sdl2::keyboard::KeyboardState) {
        self.keys[0x0] = k.is_scancode_pressed(Scancode::X);
        self.keys[0x1] = k.is_scancode_pressed(Scancode::Num1);
        self.keys[0x2] = k.is_scancode_pressed(Scancode::Num2);
        self.keys[0x3] = k.is_scancode_pressed(Scancode::Num3);
        self.keys[0x4] = k.is_scancode_pressed(Scancode::Q);
        self.keys[0x5] = k.is_scancode_pressed(Scancode::W);
        self.keys[0x6] = k.is_scancode_pressed(Scancode::E);
        self.keys[0x7] = k.is_scancode_pressed(Scancode::A);
        self.keys[0x8] = k.is_scancode_pressed(Scancode::S);
        self.keys[0x9] = k.is_scancode_pressed(Scancode::D);

        self.keys[0xA] = k.is_scancode_pressed(Scancode::Z);
        self.keys[0xB] = k.is_scancode_pressed(Scancode::C);
        self.keys[0xC] = k.is_scancode_pressed(Scancode::Num4);
        self.keys[0xD] = k.is_scancode_pressed(Scancode::R);
        self.keys[0xE] = k.is_scancode_pressed(Scancode::F);
        self.keys[0xF] = k.is_scancode_pressed(Scancode::V);
    }
    pub fn cycle(&mut self, timer_subsystem: &sdl2::TimerSubsystem) -> Result<(), String> {
        for _ in 0..CYCLES {
            let pc = self.pc as usize;
            if pc >= MEM_SIZE - 1 {
                break;
            }
            let op: u16 = (self.mem[pc] as u16) << 8 | (self.mem[pc + 1] as u16);
            if self.debug {
                let regs_dmp: String = (0..16)
                    .map(|j| format!("V{:X}:{:02X}", j, self.v[j as usize]))
                    .collect::<Vec<String>>()
                    .join(" ");
                eprintln!(
                    "PC:{:03X} {} I:{:04X} DT:{:02X} ST:{:02X} {}",
                    self.pc,
                    regs_dmp,
                    self.i,
                    self.delay,
                    self.sound,
                    opcode_str(op)
                );
            }
            self.pc += 2;

            let (x, y, n, kk, nnn) = (
                ((op >> 8) & 0xF) as usize,
                ((op >> 4) & 0xF) as usize,
                (op & 0xF) as u8,
                (op & 0xFF) as u8,
                op & 0xFFF,
            );

            match op & 0xF000 {
                0x0000 => match op {
                    0x00E0 => self.clear_d(),
                    0x00EE => {
                        self.sp -= 1;
                        self.pc = self.stack[self.sp as usize]
                    }
                    _ => {}
                },
                0x1000 => self.pc = nnn,
                0x2000 => {
                    self.stack[self.sp as usize] = self.pc;
                    self.sp += 1;
                    self.pc = nnn;
                }
                0x3000 => {
                    if self.v[x] == kk {
                        self.pc += 2
                    }
                }
                0x4000 => {
                    if self.v[x] != kk {
                        self.pc += 2
                    }
                }
                0x5000 => {
                    if self.v[x] == self.v[y] {
                        self.pc += 2
                    }
                }
                0x6000 => self.v[x] = kk,
                0x7000 => self.v[x] = self.v[x].wrapping_add(kk),
                0x8000 => match n {
                    0x0 => self.v[x] = self.v[y],
                    0x1 => self.v[x] |= self.v[y],
                    0x2 => self.v[x] &= self.v[y],
                    0x3 => self.v[x] ^= self.v[y],
                    0x4 => {
                        let (s, overflow) = self.v[x].overflowing_add(self.v[y]);
                        self.v[x] = s;
                        self.v[0xF] = if overflow { 1 } else { 0 };
                    }
                    0x5 => {
                        let (s, overflow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[x] = s;
                        self.v[0xF] = if overflow { 0 } else { 1 };
                    }
                    0x6 => {
                        self.v[0xF] = self.v[x] & 1;
                        self.v[x] >>= 1;
                    }
                    0x7 => {
                        let (s, overflow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[x] = s;
                        self.v[0xF] = if overflow { 0 } else { 1 };
                    }
                    0xE => {
                        self.v[0xF] = (self.v[x] & 0x80) >> 7;
                        self.v[x] <<= 1;
                    }
                    _ => eprintln!("Unknown 8XYN code bruh: 0x{:04X}", op),
                },
                0x9000 => {
                    if self.v[x] != self.v[y] {
                        self.pc += 2
                    }
                }
                0xA000 => self.i = nnn,
                0xB000 => self.pc = nnn.wrapping_add(self.v[0 as usize].into()),
                0xC000 => {
                    let mut rng = rand::rng();
                    self.v[x] = rng.random::<u8>() & kk;
                }
                0xD000 => {
                    self.v[0xF] = 0;
                    let start_x = self.v[x] as usize % DISP_W as usize;
                    let start_y = self.v[y] as usize % DISP_H as usize;
                    for row in 0..n as usize {
                        let sprite_byte = self.mem[self.i as usize + row];
                        let pixel_y = (start_y + row) % DISP_H as usize;
                        for bit in 0..8 {
                            if (sprite_byte & (0x80 >> bit)) != 0 {
                                let pixel_x = (start_x + bit) % DISP_W as usize;
                                let idx = pixel_y * DISP_W as usize + pixel_x;
                                if self.disp[idx] == 1 {
                                    self.v[0xF] = 1;
                                }
                                self.disp[idx] ^= 1;
                            }
                        }
                    }
                }
                0xE000 => {
                    if kk == 0x9E && self.keys[self.v[x] as usize] {
                        self.pc += 2
                    } else if kk == 0xA1 && !self.keys[self.v[x] as usize] {
                        self.pc += 2
                    }
                }
                0xF000 => match kk {
                    0x07 => self.v[x] = self.delay,
                    0x0A => {
                        let mut key_pressed = false;
                        for k in 0..16 {
                            if self.keys[k] {
                                self.v[x] = k as u8;
                                key_pressed = true;
                                break;
                            }
                        }
                        if !key_pressed {
                            self.pc -= 2
                        }
                    }
                    0x15 => self.delay = self.v[x],
                    0x18 => self.sound = self.v[x],
                    0x1E => self.i = self.i.wrapping_add(self.v[x] as u16),
                    0x29 => self.i = self.v[x] as u16 * 5,
                    0x33 => {
                        let v = self.v[x];
                        let i = self.i as usize;
                        self.mem[i] = v / 100;
                        self.mem[i + 1] = (v % 100) / 10;
                        self.mem[i + 2] = v % 10;
                    }
                    0x55 => {
                        for i in 0..=x {
                            self.mem[self.i as usize + i] = self.v[i];
                        }
                    }
                    0x65 => {
                        for i in 0..=x {
                            self.v[i] = self.mem[self.i as usize + i]
                        }
                    }
                    _ => eprintln!("unknown FXKK opcode: 0x{:04X}", op),
                },
                _ => eprintln!("unknown opcode: 0x{:04X}", op),
            }
        }
        let now = timer_subsystem.ticks64();

        if now.saturating_sub(self.last_cycle_time) >= 16 {
            if self.delay > 0 {
                self.delay -= 1;
            }
            if self.sound > 0 && !self.sound_active {
                self.sound_active = true;
                eprint!("\x07");
            } else if self.sound == 0 {
                self.sound_active = false;
            }
            if self.sound > 0 {
                self.sound -= 1;
            }
            self.last_cycle_time = now;
        }

        Ok(())
    }
}
fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let mut debug = false;
    let mut rom_path: Option<&str> = None;

    for arg in args.iter().skip(1) {
        if arg == "--debug" {
            debug = true
        } else if rom_path.is_none() {
            rom_path = Some(arg)
        } else {
            return Err(format!("unexpectedpt arg:{arg}"));
        }
    }
    let rom_path = rom_path.ok_or_else(|| "usage: <executable> [--debug] <rom>".to_string())?;

    let sdl_ctx = sdl2::init()?;
    let timer_subsystem = sdl_ctx.timer()?;
    let mut event_pump = sdl_ctx.event_pump()?;

    let mut c = Chip8::new(debug)?;
    c.load(rom_path)?;

    let frame_dur = Duration::from_millis(1000 / FPS as u64);

    'runnin: loop {
        let start_t = std::time::Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => break 'runnin,
                _ => {}
            }
        }
        let keyboard_state = event_pump.keyboard_state();
        c.input(&keyboard_state);
        c.cycle(&timer_subsystem)?;
        c.draw()?;
        let elapsed = start_t.elapsed();
        if elapsed < frame_dur {
            std::thread::sleep(frame_dur - elapsed)
        }
    }
    Ok(())
}
