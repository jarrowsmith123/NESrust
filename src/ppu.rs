use core::panic;

use crate::cart::Mirroring;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam_addr: u8,
    pub oam_data: [u8; 256],
    pub addr: AddrRegister,
    pub ctrl: ControlRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub scroll: ScrollRegister,

    pub mirroring: Mirroring,

    pub data_buf: u8,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_addr: 0,
            oam_data: [0; 256],
            addr: AddrRegister::new(),
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            scroll: ScrollRegister::new(),
            mirroring,
            data_buf: 0,
        }
    }
    pub fn write_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn write_ctrl(&mut self, value: u8) {
        self.ctrl.update(value);
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111;
        let vram_idx = mirrored_vram - 0x200;
        let name_table = vram_idx / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_idx - 0x800,
            (Mirroring::Horizontal, 2) => vram_idx - 0x400,
            (Mirroring::Horizontal, 1) => vram_idx - 0x400,
            (Mirroring::Horizontal, 3) => vram_idx - 0x800,
            _ => vram_idx,
        }
    }
    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.increment_vram_addr();

        match addr {
            0..=0x1fff => {
                let result = self.data_buf;
                self.data_buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                let result = self.data_buf;
                self.data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3eff => panic!("addr space not meant to be used here"),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize]
            }

            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!(),
        }
    }
    pub fn write_data(&mut self, data: u8) {
        let addr = self.addr.get();

        match addr {
            0..=0x1fff => {
                panic!("cant write to chr_rom")
            }
            0x2000..=0x2fff => {
                self.vram[self.mirror_vram_addr(addr) as usize] = data;
            }
            0x3000..=0x3eff => panic!("addr space not meant to be used here"),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize] = data;
            }
            0x3f00..=0x3fff => {
                self.palette_table[(addr - 0x3f00) as usize] = data;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    pub fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.status.get();
        self.status.reset_vbank_flag();
        self.addr.reset_latch();
        data
    }

    pub fn write_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }
}

pub struct AddrRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            value: (0, 0),
            hi_ptr: true,
        }
    }
    pub fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xFF) as u8;
    }

    pub fn get(&self) -> u16 {
        u16::from_be_bytes([self.value.0, self.value.1])
    }
    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }
        // since the addrregister is max 14 bits, we mirror down
        if self.get() > 0x3fff {
            self.set(self.get() & 0x3FFF)
        }
        self.hi_ptr = !self.hi_ptr;
    }
    pub fn increment(&mut self, increment: u8) {
        let lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(increment);
        if lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1)
        }
        if self.get() > 0x3FFF {
            self.set(self.get() & 0x3FFF)
        }
    }
    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }
}

pub struct ControlRegister {
    bits: u8,
}
impl ControlRegister {
    pub const NAMETABLE1: u8 = 0b00000001;
    pub const NAMETABLE2: u8 = 0b00000010;
    pub const VRAM_ADDR_INCREMENT: u8 = 0b00000100;
    pub const SPRITE_PATTERN_ADDR: u8 = 0b00001000;
    pub const BACKROUND_PATTERN_ADDR: u8 = 0b00010000;
    pub const SPRITE_SIZE: u8 = 0b00100000;
    pub const MASTER_SLAVE_SELECT: u8 = 0b01000000;
    pub const NMI_ENABLE: u8 = 0b10000000;

    pub fn new() -> Self {
        ControlRegister { bits: 0 }
    }

    pub fn vram_addr_increment(&self) -> u8 {
        // 1 for horizontal step, 32 for vertivcal
        if self.bits & Self::VRAM_ADDR_INCREMENT == 0 {
            1
        } else {
            32
        }
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}

pub struct StatusRegister {
    bits: u8,
}
impl StatusRegister {
    pub const PPU_OPEN_BUS: u8 = 0b0001_1111;
    pub const SPRITE_OVERFLOW: u8 = 0b0010_0000;
    pub const SPRITE_0_HIT: u8 = 0b0100_0000;
    pub const VBANK: u8 = 0b1000_0000;

    pub fn new() -> Self {
        StatusRegister { bits: 0 }
    }

    pub fn get(&self) -> u8 {
        self.bits
    }

    pub fn reset_vbank_flag(&mut self) {
        self.bits &= !Self::VBANK;
    }
}

pub struct MaskRegister {
    bits: u8,
}
impl MaskRegister {
    pub const GREYSCALE: u8 = 0b0000_0001;
    pub const SHOW_LEFTMOST_BCKGRND: u8 = 0b0000_0010;
    pub const SHOW_LEFTMOST_SPRITES: u8 = 0b0000_0100;
    pub const ENABLE_BCKGRND: u8 = 0b0000_1000;
    pub const ENABLE_SPRITES: u8 = 0b0001_0000;
    pub const EMPHASISE_RED: u8 = 0b0010_0000;
    pub const EMPHASISE_GREEN: u8 = 0b0100_0000;
    pub const EMPHASISE_BLUE: u8 = 0b1000_0000;

    pub fn new() -> Self {
        MaskRegister { bits: 0 }
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }

    pub fn get(&self) -> u8 {
        self.bits
    }
}

pub struct ScrollRegister {
    scroll_x: u8,
    scroll_y: u8,
    hi_ptr: bool,
}
impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            scroll_x: 0,
            scroll_y: 0,
            hi_ptr: true,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.hi_ptr {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = false;
    }
}
