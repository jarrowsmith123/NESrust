use core::panic;
use crate::cart::Mirroring;
use serde::{Deserialize, Serialize};

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

    pub scanline: u16,
    pub scanline_complete: bool,
    pub cycles: usize,

    pub nmi_interupt: Option<u8>,

    

    
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
            scanline: 0,
            scanline_complete: false,
            cycles: 0,
            nmi_interupt: None,
        }
    }

    pub fn new_empty_rom() -> Self {
        PPU::new(vec![0; 2048], Mirroring::Horizontal)
    }

    pub fn write_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
        
    }

    pub fn write_ctrl(&mut self, value: u8) {
        let nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if !nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_vblank() {
            self.nmi_interupt = Some(1);
        }
    }

    pub fn write_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111;
        let vram_idx = mirrored_vram - 0x2000;
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
            0x3000..=0x3eff => {
                let result = self.data_buf;
                self.data_buf = self.vram[self.mirror_vram_addr(addr - 0x1000) as usize];
                result
            }
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
            0x3000..=0x3eff => (),
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
        self.status.reset_vblank_flag();
        self.addr.reset_latch();
        self.scroll.reset_latch();
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

    pub fn cycle_clock(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;

        
        if self.cycles >= 341 {
            if self.is_sprite_0_hit(self.cycles) {
                self.status.set_sprite_zero_hit(true);
            }
            self.cycles -= 341;
            self.scanline += 1;

            if self.scanline <= 240 {
                self.scanline_complete = true;
            }


            if self.scanline == 241 {
                self.status.set_vblank_status(true);
                self.status.set_sprite_zero_hit(false);
                if self.ctrl.generate_vblank_nmi() {
                    self.nmi_interupt = Some(1);
                }
            }

            if self.scanline == 261 {
                if self.cycles == 1{
                    self.status.reset_vblank_flag();
                    self.status.set_sprite_zero_hit(false);
                    self.nmi_interupt = None;
                    self.scroll.scroll_x = 0;
                    self.scroll.scroll_y = 0;
                    self.ctrl.update(0);
                }
            }

            if self.scanline >= 261 {
                self.scanline = 0;
                self.nmi_interupt = None;
                self.status.set_sprite_zero_hit(false);
                self.status.reset_vblank_flag();
                return true;
                
            }
        }
        return false;
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanline as usize)
            && x <= cycle
            && self.mask.is_sprites_enabled()
            && self.mask.is_background_enabled()
    }

    pub fn poll_interupt(&mut self) -> Option<u8> {
        self.nmi_interupt.take()
    }
}


#[derive(Serialize, Deserialize, Clone)]

pub struct AddrRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            value: (0, 0), // high byte first, lo byte second
            hi_ptr: true,
        }
    }

    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }

        if self.get() > 0x3fff {
            //mirror down addr above 0x3fff
            self.set(self.get() & 0b11111111111111);
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(inc);
        if lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }
        if self.get() > 0x3fff {
            self.set(self.get() & 0b11111111111111); //mirror down addr above 0x3fff
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }
}

#[derive(Serialize, Deserialize, Clone)]

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

    pub fn sprite_pattern_addr(&self) -> u16 {
        if self.bits & Self::SPRITE_PATTERN_ADDR == 0 {
            0
        } else {
            0x1000
        }
    }

    pub fn background_pattern_addr(&self) -> u16 {
        if self.bits & Self::BACKROUND_PATTERN_ADDR == 0 {
            0
        } else {
            0x1000
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if self.bits & Self::SPRITE_SIZE == 0 {
            8
        } else {
            16
        }
    }

    pub fn generate_vblank_nmi(&self) -> bool {
        return self.bits & Self::NMI_ENABLE != 0;
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }

    pub fn get(&self) -> u8 {
        self.bits
    }

    pub fn nametable_addr(&self) -> u16 {
        match self.bits & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2c00,
            _ => panic!("not possible"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]

pub struct StatusRegister {
    bits: u8,
}
impl StatusRegister {
    pub const PPU_OPEN_BUS: u8 = 0b0001_1111;
    pub const SPRITE_OVERFLOW: u8 = 0b0010_0000;
    pub const SPRITE_0_HIT: u8 = 0b0100_0000;
    pub const VBLANK: u8 = 0b1000_0000;

    pub fn new() -> Self {
        StatusRegister { bits: 0 }
    }

    pub fn get(&self) -> u8 {
        self.bits
    }

    pub fn is_vblank(&self) -> bool {
        if self.bits & Self::VBLANK == 0 {
            return false;
        }
        return true;
    }

    pub fn set_vblank_status(&mut self, status: bool) {
        if status {
            self.bits |= Self::VBLANK;
        } else {
            self.bits &= !Self::VBLANK;
        }
    }

    pub fn set_sprite_zero_hit(&mut self, status: bool) {
        if status {
            self.bits |= Self::SPRITE_0_HIT;
        } else {
            self.bits &= !Self::SPRITE_0_HIT;
        }
    }

    pub fn reset_vblank_flag(&mut self) {
        self.bits &= !Self::VBLANK;
    }
}

#[derive(Serialize, Deserialize, Clone)]

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

    pub fn is_sprites_enabled(&self) -> bool {
        (self.bits & Self::ENABLE_SPRITES) != 0
    }

    pub fn is_background_enabled(&self) -> bool {
        (self.bits & Self::ENABLE_BCKGRND) != 0
    }

    pub fn show_background(&mut self) {
        self.bits |= Self::ENABLE_BCKGRND;
    }
}


pub struct ScrollRegister {
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub latch: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            scroll_x: 0,
            scroll_y: 0,
            latch: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.latch {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }
        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }

    pub fn get(&self) -> u16 {
        u16::from_le_bytes([self.scroll_x, self.scroll_y])
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ppu_addr(0x23);
        ppu.write_ppu_addr(0x05);
        ppu.write_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_ppu_addr(0x23);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_ppu_addr(0x21);
        ppu.write_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_ppu_addr(0x21);
        ppu.write_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ppu_addr(0x24);
        ppu.write_ppu_addr(0x05);

        ppu.write_data(0x66); //write to a

        ppu.write_ppu_addr(0x28);
        ppu.write_ppu_addr(0x05);

        ppu.write_data(0x77); //write to B

        ppu.write_ppu_addr(0x20);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_ppu_addr(0x2C);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = PPU::new(vec![0; 2048], Mirroring::Vertical);

        ppu.write_ppu_addr(0x20);
        ppu.write_ppu_addr(0x05);

        ppu.write_data(0x66); //write to A

        ppu.write_ppu_addr(0x2C);
        ppu.write_ppu_addr(0x05);

        ppu.write_data(0x77); //write to b

        ppu.write_ppu_addr(0x28);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_ppu_addr(0x24);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = PPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_ppu_addr(0x21);
        ppu.write_ppu_addr(0x23);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_ppu_addr(0x23);
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_ppu_addr(0x05);

        ppu.read_data(); //load into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = PPU::new_empty_rom();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.get() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = PPU::new_empty_rom();
        ppu.write_oam_addr(0x10);
        ppu.write_oam_data(0x66);
        ppu.write_oam_data(0x77);

        ppu.write_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = PPU::new_empty_rom();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_oam_addr(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_oam_addr(0x10);
        ppu.write_oam_addr(0x77);
        ppu.write_oam_addr(0x11);
        ppu.write_oam_addr(0x66);
    }
}
