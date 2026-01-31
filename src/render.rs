use crate::cart::Mirroring;
use crate::ppu::PPU;

pub static SYSTEM_PALLETE: [(u8, u8, u8); 64] = [
    (0x80, 0x80, 0x80),
    (0x00, 0x3D, 0xA6),
    (0x00, 0x12, 0xB0),
    (0x44, 0x00, 0x96),
    (0xA1, 0x00, 0x5E),
    (0xC7, 0x00, 0x28),
    (0xBA, 0x06, 0x00),
    (0x8C, 0x17, 0x00),
    (0x5C, 0x2F, 0x00),
    (0x10, 0x45, 0x00),
    (0x05, 0x4A, 0x00),
    (0x00, 0x47, 0x2E),
    (0x00, 0x41, 0x66),
    (0x00, 0x00, 0x00),
    (0x05, 0x05, 0x05),
    (0x05, 0x05, 0x05),
    (0xC7, 0xC7, 0xC7),
    (0x00, 0x77, 0xFF),
    (0x21, 0x55, 0xFF),
    (0x82, 0x37, 0xFA),
    (0xEB, 0x2F, 0xB5),
    (0xFF, 0x29, 0x50),
    (0xFF, 0x22, 0x00),
    (0xD6, 0x32, 0x00),
    (0xC4, 0x62, 0x00),
    (0x35, 0x80, 0x00),
    (0x05, 0x8F, 0x00),
    (0x00, 0x8A, 0x55),
    (0x00, 0x99, 0xCC),
    (0x21, 0x21, 0x21),
    (0x09, 0x09, 0x09),
    (0x09, 0x09, 0x09),
    (0xFF, 0xFF, 0xFF),
    (0x0F, 0xD7, 0xFF),
    (0x69, 0xA2, 0xFF),
    (0xD4, 0x80, 0xFF),
    (0xFF, 0x45, 0xF3),
    (0xFF, 0x61, 0x8B),
    (0xFF, 0x88, 0x33),
    (0xFF, 0x9C, 0x12),
    (0xFA, 0xBC, 0x20),
    (0x9F, 0xE3, 0x0E),
    (0x2B, 0xF0, 0x35),
    (0x0C, 0xF0, 0xA4),
    (0x05, 0xFB, 0xFF),
    (0x5E, 0x5E, 0x5E),
    (0x0D, 0x0D, 0x0D),
    (0x0D, 0x0D, 0x0D),
    (0xFF, 0xFF, 0xFF),
    (0xA6, 0xFC, 0xFF),
    (0xB3, 0xEC, 0xFF),
    (0xDA, 0xAB, 0xEB),
    (0xFF, 0xA8, 0xF9),
    (0xFF, 0xAB, 0xB3),
    (0xFF, 0xD2, 0xB0),
    (0xFF, 0xEF, 0xA6),
    (0xFF, 0xF7, 0x9C),
    (0xD7, 0xE8, 0x95),
    (0xA6, 0xED, 0xAF),
    (0xA2, 0xF2, 0xDA),
    (0x99, 0xFF, 0xFC),
    (0xDD, 0xDD, 0xDD),
    (0x11, 0x11, 0x11),
    (0x11, 0x11, 0x11),
];

pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    const WIDTH: usize = 256 * 2;
    const HEIGHT: usize = 240;

    pub fn new() -> Self {
        Frame {
            data: vec![0; (Frame::WIDTH) * (Frame::HEIGHT) * 3],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * 3 * Frame::WIDTH + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }
}

#[derive(Clone, Copy)]
struct SpritePixel {
    color_idx: u8,
    palette_idx: u8,
    priority: bool,
}

fn get_sprite_scanline(ppu: &PPU, y: usize) -> [Option<SpritePixel>; 256] {
    let mut scanline_buffer = [None; 256];
    let sprite_size = ppu.ctrl.sprite_size() as usize;

    for i in (0..ppu.oam_data.len()).step_by(4) {
        let tile_y = ppu.oam_data[i] as usize;

        if y >= tile_y && y < tile_y + sprite_size {
            let tile_idx = ppu.oam_data[i + 1] as u16;
            let attr = ppu.oam_data[i + 2];
            let tile_x = ppu.oam_data[i + 3] as usize;

            let priority = (attr >> 5) & 1 == 1;
            let flip_h = (attr >> 6) & 1 == 1;
            let flip_v = (attr >> 7) & 1 == 1;
            let palette_idx = attr & 0b11;

            let bank: u16;
            let tile_ptr: u16;

            if sprite_size == 16 {
                bank = if tile_idx & 1 == 0 { 0 } else { 0x1000 };
                tile_ptr = tile_idx & 0xFE;
            } else {
                bank = ppu.ctrl.sprite_pattern_addr();
                tile_ptr = tile_idx;
            }

            let mut row = y - tile_y;
            if flip_v {
                row = sprite_size - 1 - row;
            }

            let final_tile_ptr = if row >= 8 && sprite_size == 16 {
                tile_ptr + 1
            } else {
                tile_ptr
            };

            let lo_byte = ppu.chr_rom[(bank + final_tile_ptr * 16 + (row % 8) as u16) as usize];
            let hi_byte = ppu.chr_rom[(bank + final_tile_ptr * 16 + (row % 8) as u16 + 8) as usize];

            for x in 0..8 {
                if tile_x + x >= 256 {
                    continue;
                }

                let current_x = if flip_h { 7 - x } else { x };
                let bit_mask = 1 << (7 - current_x);

                let pixel_val =
                    ((lo_byte & bit_mask) > 0) as u8 | (((hi_byte & bit_mask) > 0) as u8) << 1;

                if pixel_val != 0 && scanline_buffer[tile_x + x].is_none() {
                    scanline_buffer[tile_x + x] = Some(SpritePixel {
                        color_idx: pixel_val,
                        palette_idx,
                        priority,
                    });
                }
            }
        }
    }
    scanline_buffer
}

fn get_system_color(ppu: &PPU, palette_idx: u8, color_idx: u8, is_sprite: bool) -> (u8, u8, u8) {
    let base_address = if is_sprite { 0x3F10 } else { 0x3F00 };
    let palette_start = base_address + (palette_idx * 4) as usize;
    let final_idx = if color_idx == 0 { 0 } else { color_idx };
    let pal_index = ppu.palette_table[(palette_start + final_idx as usize) - 0x3F00];
    SYSTEM_PALLETE[pal_index as usize]
}

fn get_bg_pixel(ppu: &PPU, x: usize, y: usize) -> (u8, u8) {
    if !ppu.mask.is_background_enabled() {
        return (0, 0);
    }

    let scroll_x = ppu.scroll.scroll_x as usize;
    let scroll_y = ppu.scroll.scroll_y as usize;
    let base_nt = ppu.ctrl.nametable_addr();

    let base_nt_idx = match base_nt {
        0x2000 => 0,
        0x2400 => 1,
        0x2800 => 2,
        0x2C00 => 3,
        _ => 0,
    };

    let coarse_x_offset = (base_nt_idx % 2) * 256;
    let coarse_y_offset = (base_nt_idx / 2) * 240;

    let abs_x = (coarse_x_offset + scroll_x + x) % 512;
    let abs_y = (coarse_y_offset + scroll_y + y) % 480;

    let current_nt_idx = (abs_x / 256) + (abs_y / 240) * 2;
    let local_x = abs_x % 256;
    let local_y = abs_y % 240;

    let vram_slice = match (&ppu.mirroring, current_nt_idx) {
        (Mirroring::Vertical, 0) | (Mirroring::Vertical, 2) => &ppu.vram[0..0x400],
        (Mirroring::Vertical, 1) | (Mirroring::Vertical, 3) => &ppu.vram[0x400..0x800],
        (Mirroring::Horizontal, 0) | (Mirroring::Horizontal, 1) => &ppu.vram[0..0x400],
        (Mirroring::Horizontal, 2) | (Mirroring::Horizontal, 3) => &ppu.vram[0x400..0x800],
        _ => &ppu.vram[0..0x400],
    };

    let tile_col = local_x / 8;
    let tile_row = local_y / 8;
    let tile_idx = vram_slice[tile_row * 32 + tile_col] as u16;

    let attr_offset = 0x3C0 + (tile_row / 4) * 8 + (tile_col / 4);
    let attr_byte = vram_slice[attr_offset];

    let shift = match (tile_col % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => 0,
        (1, 0) => 2,
        (0, 1) => 4,
        (1, 1) => 6,
        _ => 0,
    };
    let palette_idx = (attr_byte >> shift) & 0b11;

    let bank = ppu.ctrl.background_pattern_addr();
    let fine_y = local_y % 8;
    let lo = ppu.chr_rom[(bank + tile_idx * 16 + fine_y as u16) as usize];
    let hi = ppu.chr_rom[(bank + tile_idx * 16 + fine_y as u16 + 8) as usize];

    let fine_x = local_x % 8;
    let bit_mask = 1 << (7 - fine_x);

    let pixel_val = ((lo & bit_mask) > 0) as u8 | (((hi & bit_mask) > 0) as u8) << 1;

    (pixel_val, palette_idx)
}

pub fn render_scanline(ppu: &PPU, frame: &mut Frame, y: usize) {
    let sprite_scanline = get_sprite_scanline(ppu, y);
    let backdrop_color = SYSTEM_PALLETE[ppu.palette_table[0] as usize];

    for x in 0..256 {
        let (bg_pixel, bg_palette) = get_bg_pixel(ppu, x, y);
        let sprite_pixel = sprite_scanline[x];
        let final_color;

        match (bg_pixel, sprite_pixel) {
            (0, None) => final_color = backdrop_color,
            (1..=3, None) => final_color = get_system_color(ppu, bg_palette, bg_pixel, false),
            (0, Some(sprite)) => {
                final_color = get_system_color(ppu, sprite.palette_idx, sprite.color_idx, true)
            }
            (1..=3, Some(sprite)) => {
                if sprite.priority {
                    final_color = get_system_color(ppu, bg_palette, bg_pixel, false);
                } else {
                    final_color = get_system_color(ppu, sprite.palette_idx, sprite.color_idx, true);
                }
            }
            _ => final_color = backdrop_color,
        }
        frame.set_pixel(x, y, final_color);
    }
}

pub fn render(ppu: &PPU, frame: &mut Frame) {
    for y in 0..240 {
        render_scanline(ppu, frame, y);
    }
}
