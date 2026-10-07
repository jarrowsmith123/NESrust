use crate::cart::ROM;
use crate::controller::Joypad;
use crate::ppu::PPU;
use crate::cpu::CPU;


use serde::{Deserialize,Serialize};



const RAM: u16 = 0x0000;
const RAM_MIRROR_END: u16 = 0x1FFF;
const PPU_REGISTER_MIRROR_END: u16 = 0x3FFF;


pub struct Bus {
    vram: [u8; 2048],
    prg_rom: Vec<u8>,
    pub ppu: PPU,

    cycles: usize,
    pub joypad1: Joypad,
    pub joypad2: Joypad,
}

impl Bus {
    pub fn new(rom: ROM) -> Bus

    {
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);
        Bus {
            vram: [0; 2048],
            prg_rom: rom.prg_rom,
            ppu,
            cycles: 0,
            joypad1: Joypad::new(),
            joypad2: Joypad::new(),
        }
    }

    pub fn cycle_clock(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        self.ppu.cycle_clock(cycles * 3);
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRROR_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.vram[mirror_down_addr as usize]
            }
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),

            0x2008..=PPU_REGISTER_MIRROR_END => {
                let _mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_read(_mirror_down_addr)
            }
            0x4016 => self.joypad1.read(),

            0x8000..=0xFFFF => self.read_prg_rom(addr),
            _ => 0,
        }
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM..=RAM_MIRROR_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.vram[mirror_down_addr as usize] = data;
            }
            0x2000 => {
                self.ppu.write_ctrl(data);
            }
            0x2001 => {
                self.ppu.write_mask(data);
            }
            0x2003 => {
                self.ppu.write_oam_addr(data);
            }
            0x2004 => {
                self.ppu.write_oam_data(data);
            }
            0x2005 => {
                self.ppu.write_scroll(data);
            }
            0x2006 => {
                self.ppu.write_ppu_addr(data);
            }
            0x2007 => {
                self.ppu.write_data(data);
            }
            0x2008..=PPU_REGISTER_MIRROR_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_write(mirror_down_addr, data);
            }
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }

                self.ppu.write_oam_dma(&buffer);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(255);
                self.ppu.cycle_clock(9);
            }
            0x4016 => {
                self.joypad1.write(data);
            }

            0x8000..=0xFFFF => {}
            _ => {}
        }
    }

    pub fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.prg_rom[addr as usize]
    }

    pub fn poll_interupt(&mut self) -> Option<u8> {
        self.ppu.poll_interupt()
    }
}


