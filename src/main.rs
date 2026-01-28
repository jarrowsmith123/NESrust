pub mod bus;
pub mod cart;
pub mod cpu;
pub mod ppu;
pub mod render;
pub mod controller;

use bus::Bus;
use cart::ROM;
use cpu::CPU;
use ppu::PPU;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use crate::render::Frame;
use std::collections::HashMap;



fn main() {
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, 0b00100000);
    key_map.insert(Keycode::Up, 0b00010000);
    key_map.insert(Keycode::Right, 0b10000000);
    key_map.insert(Keycode::Left, 0b01000000);
    key_map.insert(Keycode::Space, 0b00000100);
    key_map.insert(Keycode::Return, 0b00001000);
    key_map.insert(Keycode::A, 0b00000001);
    key_map.insert(Keycode::S, 0b00000010);
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(2.0, 2.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    //load the game
    let bytes: Vec<u8> = std::fs::read("smb.nes").unwrap();
    let rom = ROM::new(&bytes).unwrap();

    let mut frame = Frame::new();


    let bus = Bus::new(rom, move |ppu: &PPU, joypad: &mut controller::Joypad| {

        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 2 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),


                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, true);
                    }
            
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }

                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();
}
