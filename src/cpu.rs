use crate::bus::Bus;

pub struct CPU {
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus,
}

/// # Status Register (P) http://wiki.nesdev.com/w/index.php/Status_flags
///
///  7 6 5 4 3 2 1 0
///  N V _ B D I Z C
///  | |   | | | | +--- Carry Flag
///  | |   | | | +----- Zero Flag
///  | |   | | +------- Interrupt Disable
///  | |   | +--------- Decimal Mode (not used on NES)
///  | |   +----------- Break Command
///  | +--------------- Overflow Flag
///  +----------------- Negative Flag
///
///

#[derive(Clone, Copy)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    NoneAddressing,
}

impl CPU {
    pub fn new(bus: Bus) -> CPU {
        CPU {
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            stack_pointer: 0xFD,
            bus,
        }
    }

    pub fn get_trace(&mut self) -> String {
        let pc = self.program_counter;
        let opcode = self.mem_read(pc);

        // This is most of them i think
        let len = match opcode {
            // --- 3-BYTE INSTRUCTIONS ---
            // Absolute, Absolute X/Y, JMP, JSR, and Indirect JMP
            0x0D | 0x0E | 0x1D | 0x1E | 0x20 | 0x2C | 0x2D | 0x2E | 0x3D | 0x3E | 0x4C | 0x4D
            | 0x4E | 0x5D | 0x5E | 0x6C | 0x6D | 0x6E | 0x7D | 0x7E | 0x8C | 0x8D | 0x8E | 0x9D
            | 0xAC | 0xAD | 0xAE | 0xBC | 0xBD | 0xBE | 0xCD | 0xCE | 0xDD | 0xDE | 0xEC | 0xED
            | 0xEE | 0xFD | 0xFE => 3,

            // --- 2-BYTE INSTRUCTIONS ---
            // Immediate, Zero Page, Zero Page X/Y, Indirect X/Y, and Relative (Branches)
            0x01 | 0x05 | 0x06 | 0x09 | 0x10 | 0x11 | 0x12 | 0x15 | 0x16 | 0x21 | 0x24 | 0x25
            | 0x26 | 0x29 | 0x30 | 0x31 | 0x35 | 0x36 | 0x41 | 0x45 | 0x46 | 0x49 | 0x50 | 0x51
            | 0x55 | 0x56 | 0x61 | 0x65 | 0x66 | 0x69 | 0x70 | 0x71 | 0x75 | 0x76 | 0x80 | 0x81
            | 0x84 | 0x85 | 0x86 | 0x90 | 0x91 | 0x94 | 0x95 | 0x96 | 0xA0 | 0xA1 | 0xA2 | 0xA4
            | 0xA5 | 0xA6 | 0xA9 | 0xB0 | 0xB1 | 0xB4 | 0xB5 | 0xB6 | 0xB9 | 0xC0 | 0xC1 | 0xC4
            | 0xC5 | 0xC6 | 0xC9 | 0xD0 | 0xD1 | 0xD5 | 0xD6 | 0xE0 | 0xE1 | 0xE4 | 0xE5 | 0xE6
            | 0xE9 | 0xF0 | 0xF1 | 0xF5 | 0xF6 => 2,

            // --- 1-BYTE INSTRUCTIONS ---
            // Implied, Accumulator, and Stack operations
            _ => 1,
        };

        // Collect the bytes into a string
        let mut bytes = Vec::new();
        for i in 0..len {
            bytes.push(format!("{:02X}", self.mem_read(pc.wrapping_add(i as u16))));
        }
        let hex_bytes = bytes.join(" ");

        format!(
            "{:04X}  {:9} A:{:02X} X:{:02X} Y:{:02X} P:{:08b} SP:{:02X} PA:{:02X} PM:{:08b} PC:{:08b} PSC:{:02X} PS:{:08b} PPU: {:03}:{:03}",
            pc,
            hex_bytes, // This will be like "A9 01"
            self.accumulator,
            self.register_x,
            self.register_y,
            self.status,
            self.stack_pointer,
            self.bus.ppu.addr.get(),
            self.bus.ppu.mask.get(),
            self.bus.ppu.ctrl.get(),
            self.bus.ppu.scroll.get(),
            self.bus.ppu.status.get(),
            self.bus.ppu.scanline,
            self.bus.ppu.cycles,
        )
    }

    fn get_address_from_opcode(&mut self, mode: AddressingMode) -> (u16, bool) {
        match mode {
            // Immediatet - uses no address from opcode
            AddressingMode::Immediate => {
                let addr = self.program_counter;
                self.program_counter += 1;
                (addr, false)
            }
            // Zero page - only one byte address
            AddressingMode::ZeroPage => {
                let addr = self.mem_read(self.program_counter) as u16;
                self.program_counter += 1;
                (addr, false)
            }
            // Absolute - 2 byte address
            AddressingMode::Absolute => {
                let addr = self.mem_read_u16(self.program_counter);
                self.program_counter += 2;
                (addr, false)
            }
            // Zero page X - one byte + value stored in x register
            AddressingMode::ZeroPageX => {
                let start = self.mem_read(self.program_counter);
                self.program_counter += 1;
                (start.wrapping_add(self.register_x) as u16, false)
            }
            // Zero page Y - same as above but for register y
            AddressingMode::ZeroPageY => {
                let start = self.mem_read(self.program_counter);
                self.program_counter += 1;
                (start.wrapping_add(self.register_y) as u16, false)
            }
            // Absolute X - read next 2 bytes + register x
            AddressingMode::AbsoluteX => {
                let start = self.mem_read_u16(self.program_counter);
                self.program_counter += 2;
                let addr = start.wrapping_add(self.register_x as u16);

                let page_crossed = (start & 0xFF00) != (addr & 0xFF00);
                (addr, page_crossed)
            }
            // Absolute Y - same as above for register y
            AddressingMode::AbsoluteY => {
                let start = self.mem_read_u16(self.program_counter);
                self.program_counter += 2;
                let addr = start.wrapping_add(self.register_y as u16);

                let page_crossed = (start & 0xFF00) != (addr & 0xFF00);
                (addr, page_crossed)
            }
            // Indirect X - read byte, add X and then look up 16 byte address at that location
            AddressingMode::IndirectX => {
                let start = self.mem_read(self.program_counter);
                self.program_counter += 1;

                let ptr = start.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);

                (u16::from_le_bytes([lo, hi]), false)
            }
            // Indirect Y - read byte, look up 16 byte address then add Y
            AddressingMode::IndirectY => {
                let start = self.mem_read(self.program_counter);
                self.program_counter += 1;

                let lo = self.mem_read(start as u16);
                let hi = self.mem_read(start.wrapping_add(1) as u16);
                let addr = u16::from_le_bytes([lo, hi]);
                let final_addr = addr.wrapping_add(self.register_y as u16);
                let page_crossed = (addr & 0xFF00) != (final_addr & 0xFF00);
                (final_addr, page_crossed)
            }
            _ => {
                todo!()
            }
        }
    }

    // ----------------  Instructions   -------------------

    fn lda(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let value = self.mem_read(addr);
        self.accumulator = value;
        if wrapping {
            self.bus.cycle_clock(1);
        }

        self.update_flags(self.accumulator);
    }

    fn ldx(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let value = self.mem_read(addr);
        self.register_x = value;
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.register_x);
    }

    fn ldy(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let value = self.mem_read(addr);
        self.register_y = value;
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.register_y);
    }

    fn sta(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        self.mem_write(addr, self.accumulator);
    }

    fn stx(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        self.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        self.mem_write(addr, self.register_y);
    }

    fn adc(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let value = self.mem_read(addr);
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.add_to_accumulator(value);
        self.update_flags(self.accumulator);
    }

    fn sbc(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let value = self.mem_read(addr);
        self.add_to_accumulator(!value);
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.accumulator);
    }

    fn tax(&mut self) {
        self.register_x = self.accumulator;
        self.update_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.accumulator;
        self.update_flags(self.register_y);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_flags(self.register_x);
    }

    fn clc(&mut self) {
        self.status &= !0b0000_0001
    }
    fn sec(&mut self) {
        self.status |= 0b0000_0001
    }
    fn cli(&mut self) {
        self.status &= !0b0000_0100;
    }
    fn sei(&mut self) {
        self.status |= 0b0000_0100;
    }
    fn clv(&mut self) {
        self.status &= !0b0100_0000;
    }
    fn cld(&mut self) {
        self.status &= !0b0000_1000;
    }
    fn sed(&mut self) {
        self.status |= 0b0000_1000;
    }

    fn and(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        self.accumulator &= data;
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.accumulator);
    }

    fn eor(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        self.accumulator ^= data;
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.accumulator);
    }

    fn ora(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        self.accumulator |= data;
        if wrapping {
            self.bus.cycle_clock(1);
        }
        self.update_flags(self.accumulator);
    }

    fn bit(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        let result = self.accumulator & data;
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= !0b0000_0010;
        }
        let status_bits = data & 0b1100_0000;
        self.status &= 0b0011_1111;
        self.status |= status_bits;
    }

    fn cmp(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        self.compare(addr, self.accumulator);
        if wrapping {
            self.bus.cycle_clock(1);
        }
    }
    fn cpx(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        self.compare(addr, self.register_x);
        if wrapping {
            self.bus.cycle_clock(1);
        }
    }
    fn cpy(&mut self, mode: AddressingMode) {
        let (addr, wrapping) = self.get_address_from_opcode(mode);
        self.compare(addr, self.register_y);
        if wrapping {
            self.bus.cycle_clock(1);
        }
    }

    fn txa(&mut self) {
        self.accumulator = self.register_x;
        self.update_flags(self.accumulator);
    }
    fn tya(&mut self) {
        self.accumulator = self.register_y;
        self.update_flags(self.accumulator);
    }
    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_flags(self.register_x);
    }
    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn inc(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        let data = data.wrapping_add(1);
        self.mem_write(addr, data);
        self.update_flags(data);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_flags(self.register_y);
    }

    fn dec(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        let data = data.wrapping_sub(1);
        self.mem_write(addr, data);
        self.update_flags(data);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_flags(self.register_y);
    }

    fn asl_accumulator(&mut self) {
        let data = self.accumulator;
        if (data & 0b1000_0000) == 0b1000_0000 {
            // set carry
            self.sec();
        } else {
            // clear carry
            self.clc();
        }
        let result = data << 1;
        self.update_flags(result);
        self.accumulator = result;
    }

    fn asl(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        if (data & 0b1000_0000) == 0b1000_0000 {
            // set carry
            self.sec();
        } else {
            // clear carry
            self.clc();
        }
        let result = data << 1;
        self.update_flags(result);
        self.mem_write(addr, result);
    }

    fn lsr_accumulator(&mut self) {
        let data = self.accumulator;
        if (data & 0b0000_0001) == 0b0000_0001 {
            // set carry
            self.sec();
        } else {
            // clear carry
            self.clc();
        }
        let result = data >> 1;
        self.update_flags(result);
        self.accumulator = result;
    }

    fn lsr(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let data = self.mem_read(addr);
        if (data & 0b0000_0001) == 0b0000_0001 {
            // set carry
            self.sec();
        } else {
            // clear carry
            self.clc();
        }
        let result = data >> 1;
        self.update_flags(result);
        self.mem_write(addr, result);
    }

    fn rol_accumulator(&mut self) {
        let mut data = self.accumulator;
        if (data & 0b1000_0000) == 0b1000_0000 {
            data = data << 1;
            data |= 0b0000_0001 & self.status;
            self.sec();
        } else {
            data = data << 1;
            data |= 0b0000_0001 & self.status;
            self.clc();
        }
        self.accumulator = data;
        self.update_flags(data);
    }
    fn rol(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let mut data = self.mem_read(addr);
        if (data & 0b1000_0000) == 0b1000_0000 {
            data = data << 1;
            data |= 0b0000_0001 & self.status;
            self.sec();
        } else {
            data = data << 1;
            data |= 0b0000_0001 & self.status;
            self.clc();
        }
        self.mem_write(addr, data);
        self.update_flags(data);
    }
    fn ror_accumulator(&mut self) {
        let mut data = self.accumulator;
        if (data & 0b0000_0001) == 0b0000_0001 {
            data >>= 1;
            if (0b0000_0001 & self.status) == 1 {
                data |= 0b1000_0000;
            }
            self.sec();
        } else {
            data >>= 1;
            if (0b0000_0001 & self.status) == 1 {
                data |= 0b1000_0000;
            }
            self.clc();
        }
        self.accumulator = data;
        self.update_flags(data);
    }
    fn ror(&mut self, mode: AddressingMode) {
        let (addr, _wrapping) = self.get_address_from_opcode(mode);
        let mut data = self.mem_read(addr);
        if (data & 0b0000_0001) == 0b0000_0001 {
            data >>= 1;
            if (0b0000_0001 & self.status) == 1 {
                data |= 0b1000_0000;
            }
            self.sec();
        } else {
            data >>= 1;
            if (0b0000_0001 & self.status) == 1 {
                data |= 0b1000_0000;
            }
            self.clc();
        }
        self.mem_write(addr, data);
        self.update_flags(data);
    }

    fn bcc(&mut self) {
        self.branch((self.status & 0b0000_0001) == 0);
    }

    fn bcs(&mut self) {
        self.branch((self.status & 0b0000_0001) != 0);
    }

    fn beq(&mut self) {
        self.branch((self.status & 0b0000_0010) != 0);
    }

    fn bne(&mut self) {
        self.branch((self.status & 0b0000_0010) == 0);
    }

    fn bpl(&mut self) {
        self.branch((self.status & 0b1000_0000) == 0);
    }

    fn bmi(&mut self) {
        self.branch((self.status & 0b1000_0000) != 0);
    }

    fn bvc(&mut self) {
        self.branch((self.status & 0b0100_0000) == 0);
    }

    fn bvs(&mut self) {
        self.branch((self.status & 0b0100_0000) != 0);
    }

    fn jmp(&mut self, mode: AddressingMode) {
        ///// JMP INDIRECT HAS A BUG THAT MIGHT NEED ADDING!!!!!!!!!!!!

        match mode {
            AddressingMode::IndirectX => {
                let start = self.mem_read_u16(self.program_counter);
                let lo = self.mem_read(start);
                let hi = if (start & 0x00FF) == 0x00FF {
                    self.mem_read(start & 0xFF00)
                } else {
                    self.mem_read(start.wrapping_add(1))
                };
                self.program_counter = u16::from_le_bytes([lo, hi]);
            }
            AddressingMode::Absolute => {
                let addr = self.mem_read_u16(self.program_counter);
                self.program_counter = addr;
            }
            _ => {}
        }
    }

    fn jsr(&mut self) {
        let return_addr = self.program_counter + 1;
        let hi = (return_addr >> 8) as u8;
        let lo = (return_addr & 0xFF) as u8;
        let jmp_addr = self.mem_read_u16(self.program_counter);
        self.push(hi);
        self.push(lo);

        self.program_counter = jmp_addr;
    }

    fn rts(&mut self) {
        let lo = self.pop();
        let hi = self.pop();
        let addr = u16::from_le_bytes([lo, hi]);
        self.program_counter = addr.wrapping_add(1);
    }

    fn rti(&mut self) {
        self.plp();
        let lo = self.pop();
        let hi = self.pop();
        self.program_counter = u16::from_le_bytes([lo, hi]);
    }
    fn pha(&mut self) {
        self.push(self.accumulator);
    }
    fn pla(&mut self) {
        self.accumulator = self.pop();
        self.update_flags(self.accumulator);
    }
    fn php(&mut self) {
        // set B flag here
        self.push(self.status | 0b0011_0000)
    }
    fn plp(&mut self) {
        let result = self.pop() & 0b1100_1111;
        let status_masked = self.status & 0b0011_0000;
        self.status = status_masked | result;
    }

    // ----------------------------------------------------

    fn add_to_accumulator(&mut self, data: u8) {
        let carry = self.status & 1;
        let sum = self.accumulator as u16 + data as u16 + carry as u16;
        if sum > 0xFF {
            // set carry flag to 1
            self.sec();
        } else {
            // set carry flag to 0
            self.clc();
        }

        let result = sum as u8;

        if (data ^ result) & (result ^ self.accumulator) & 0x80 != 0 {
            // set overflow flag to 1
            self.status |= 0b0100_0000
        } else {
            // set overflow flag to 0
            self.status &= !0b0100_0000
        }
        self.accumulator = result;
    }

    fn branch(&mut self, condition: bool) {
        let offset = self.mem_read(self.program_counter) as i8;
        self.program_counter = self.program_counter.wrapping_add(1);
        if condition {
            self.bus.cycle_clock(1);
            let jump_addr = self.program_counter.wrapping_add(offset as u16);
            self.program_counter = jump_addr;
            // check to see if page is crossed
            if (self.program_counter & 0xFF00) != (jump_addr & 0xFF00) {
                self.bus.cycle_clock(1);
            }
        }
    }

    fn compare(&mut self, addr: u16, compare_with: u8) {
        // Subtracts data at addr from compare_with and sets correct flag
        let data = self.mem_read(addr);
        // ssets carry flag
        if data <= compare_with {
            self.status |= 0b0000_0001;
        } else {
            self.status &= !0b0000_0001;
        }
        let result = compare_with.wrapping_sub(data);
        // set zero and negative flags
        self.update_flags(result);
    }

    fn push(&mut self, data: u8) {
        self.mem_write(0x0100 + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read(0x0100 + self.stack_pointer as u16)
    }

    pub fn mem_read(&mut self, addr: u16) -> u8 {
        // gets the byte at a given addr
        self.bus.mem_read(addr)
    }

    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        // gets the 2 bytes of a given addr
        let lo = self.bus.mem_read(addr);
        let hi = self.bus.mem_read(addr.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data);
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.mem_write(addr, bytes[0]);
        self.mem_write(addr.wrapping_add(1), bytes[1]);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(i, program[i as usize]);
        }
        self.mem_write_u16(0xFFFC, 0x0000);
    }

    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0b100100;
        self.stack_pointer = 0xFD;

        self.program_counter = self.mem_read_u16(0xFFFC)
    }

    fn update_flags(&mut self, result: u8) {
        if result == 0 {
            // Set zero flag if param zero
            self.status |= 0b000_0010
        } else {
            self.status &= 0b1111_1101;
        }
        if result & 0b1000_0000 != 0 {
            // Set negative flag if param is negative
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }

    fn interupt_nmi(&mut self) {
        let hi = (self.program_counter >> 8) as u8;
        let lo = (self.program_counter & 0xFF) as u8;
        self.push(hi);
        self.push(lo);

        self.push(self.status | 0b0010_0000);
        self.sei();

        self.bus.cycle_clock(2);
        self.program_counter = self.mem_read_u16(0xFFFA)
    }

    fn interupt_brk(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);
        let hi = (self.program_counter >> 8) as u8;
        let lo = (self.program_counter & 0xFF) as u8;
        self.push(hi);
        self.push(lo);
        // push flag plus B flag
        self.push(self.status | 0b0011_0000);
        self.sei();

        self.bus.cycle_clock(7);
        self.program_counter = self.mem_read_u16(0xFFFE);
    }

    pub fn step(&mut self) {
            if let Some(_nmi) = self.bus.poll_interupt() {
                self.interupt_nmi()
            }

            //println!("{}", self.get_trace());
            let opcode = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match opcode {
                // LDA
                0xA9 => {
                    self.lda(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xA5 => {
                    self.lda(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xB5 => {
                    self.lda(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0xAD => {
                    self.lda(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0xBD => {
                    self.lda(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xB9 => {
                    self.lda(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xA1 => {
                    self.lda(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0xB1 => {
                    self.lda(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // LDX
                0xA2 => {
                    self.ldx(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xA6 => {
                    self.ldx(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xB6 => {
                    self.ldx(AddressingMode::ZeroPageY);
                    self.bus.cycle_clock(4);
                }
                0xAE => {
                    self.ldx(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0xBE => {
                    self.ldx(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }

                // LDY
                0xA0 => {
                    self.ldy(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xA4 => {
                    self.ldy(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xB4 => {
                    self.ldy(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0xAC => {
                    self.ldy(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0xBC => {
                    self.ldy(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }

                // STA
                0x85 => {
                    self.sta(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x95 => {
                    self.sta(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x8D => {
                    self.sta(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0x9D => {
                    self.sta(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(5);
                }
                0x99 => {
                    self.sta(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(5);
                }
                0x81 => {
                    self.sta(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0x91 => {
                    self.sta(AddressingMode::IndirectY);
                    self.bus.cycle_clock(6);
                }

                // STX
                0x86 => {
                    self.stx(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x96 => {
                    self.stx(AddressingMode::ZeroPageY);
                    self.bus.cycle_clock(4);
                }
                0x8E => {
                    self.stx(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }

                // STY
                0x84 => {
                    self.sty(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x94 => {
                    self.sty(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x8C => {
                    self.sty(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }

                // ADC
                0x69 => {
                    self.adc(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0x65 => {
                    self.adc(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x75 => {
                    self.adc(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x6D => {
                    self.adc(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0x7D => {
                    self.adc(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x79 => {
                    self.adc(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x61 => {
                    self.adc(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0x71 => {
                    self.adc(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // SBC
                0xE9 => {
                    self.sbc(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xE5 => {
                    self.sbc(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xF5 => {
                    self.sbc(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0xED => {
                    self.sbc(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0xFD => {
                    self.sbc(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xF9 => {
                    self.sbc(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xE1 => {
                    self.sbc(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0xF1 => {
                    self.sbc(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // AND
                0x29 => {
                    self.and(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0x25 => {
                    self.and(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x35 => {
                    self.and(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x2D => {
                    self.and(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0x3D => {
                    self.and(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x39 => {
                    self.and(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x21 => {
                    self.and(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0x31 => {
                    self.and(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // EOR
                0x49 => {
                    self.eor(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0x45 => {
                    self.eor(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x55 => {
                    self.eor(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x4D => {
                    self.eor(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0x5D => {
                    self.eor(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x59 => {
                    self.eor(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x41 => {
                    self.eor(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0x51 => {
                    self.eor(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // ORA
                0x09 => {
                    self.ora(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0x05 => {
                    self.ora(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x15 => {
                    self.ora(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0x0D => {
                    self.ora(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0x1D => {
                    self.ora(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x19 => {
                    self.ora(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0x01 => {
                    self.ora(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0x11 => {
                    self.ora(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // BIT
                0x24 => {
                    self.bit(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0x2C => {
                    self.bit(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }

                // CMP
                0xC9 => {
                    self.cmp(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xC5 => {
                    self.cmp(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xD5 => {
                    self.cmp(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(4);
                }
                0xCD => {
                    self.cmp(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }
                0xDD => {
                    self.cmp(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xD9 => {
                    self.cmp(AddressingMode::AbsoluteY);
                    self.bus.cycle_clock(4); // +1 if page crossed
                }
                0xC1 => {
                    self.cmp(AddressingMode::IndirectX);
                    self.bus.cycle_clock(6);
                }
                0xD1 => {
                    self.cmp(AddressingMode::IndirectY);
                    self.bus.cycle_clock(5); // +1 if page crossed
                }

                // CPX
                0xE0 => {
                    self.cpx(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xE4 => {
                    self.cpx(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xEC => {
                    self.cpx(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }

                // CPY
                0xC0 => {
                    self.cpy(AddressingMode::Immediate);
                    self.bus.cycle_clock(2);
                }
                0xC4 => {
                    self.cpy(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(3);
                }
                0xCC => {
                    self.cpy(AddressingMode::Absolute);
                    self.bus.cycle_clock(4);
                }

                // INC
                0xE6 => {
                    self.inc(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0xF6 => {
                    self.inc(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0xEE => {
                    self.inc(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0xFE => {
                    self.inc(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // INX
                0xE8 => {
                    self.inx();
                    self.bus.cycle_clock(2);
                }

                // INY
                0xC8 => {
                    self.iny();
                    self.bus.cycle_clock(2);
                }

                // DEC
                0xC6 => {
                    self.dec(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0xD6 => {
                    self.dec(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0xCE => {
                    self.dec(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0xDE => {
                    self.dec(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // DEX
                0xCA => {
                    self.dex();
                    self.bus.cycle_clock(2);
                }

                // DEY
                0x88 => {
                    self.dey();
                    self.bus.cycle_clock(2);
                }

                // ASL
                0x0A => {
                    self.asl_accumulator();
                    self.bus.cycle_clock(2);
                }
                0x06 => {
                    self.asl(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0x16 => {
                    self.asl(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0x0E => {
                    self.asl(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0x1E => {
                    self.asl(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // LSR
                0x4A => {
                    self.lsr_accumulator();
                    self.bus.cycle_clock(2);
                }
                0x46 => {
                    self.lsr(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0x56 => {
                    self.lsr(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0x4E => {
                    self.lsr(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0x5E => {
                    self.lsr(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // ROL
                0x2A => {
                    self.rol_accumulator();
                    self.bus.cycle_clock(2);
                }
                0x26 => {
                    self.rol(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0x36 => {
                    self.rol(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0x2E => {
                    self.rol(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0x3E => {
                    self.rol(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // ROR
                0x6A => {
                    self.ror_accumulator();
                    self.bus.cycle_clock(2);
                }
                0x66 => {
                    self.ror(AddressingMode::ZeroPage);
                    self.bus.cycle_clock(5);
                }
                0x76 => {
                    self.ror(AddressingMode::ZeroPageX);
                    self.bus.cycle_clock(6);
                }
                0x6E => {
                    self.ror(AddressingMode::Absolute);
                    self.bus.cycle_clock(6);
                }
                0x7E => {
                    self.ror(AddressingMode::AbsoluteX);
                    self.bus.cycle_clock(7);
                }

                // BCC
                0x90 => {
                    self.bcc();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BCS
                0xB0 => {
                    self.bcs();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BEQ
                0xF0 => {
                    self.beq();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BMI
                0x30 => {
                    self.bmi();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BNE
                0xD0 => {
                    self.bne();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BPL
                0x10 => {
                    self.bpl();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BVC
                0x50 => {
                    self.bvc();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // BVS
                0x70 => {
                    self.bvs();
                    self.bus.cycle_clock(2); // +1 if taken, +2 if crossed
                }

                // JMP
                0x4C => {
                    self.jmp(AddressingMode::Absolute);
                    self.bus.cycle_clock(3);
                }
                0x6C => {
                    self.jmp(AddressingMode::IndirectX);
                    self.bus.cycle_clock(5);
                }

                // JSR
                0x20 => {
                    self.jsr();
                    self.bus.cycle_clock(6);
                }

                // RTS
                0x60 => {
                    self.rts();
                    self.bus.cycle_clock(6);
                }

                // RTI
                0x40 => {
                    self.rti();
                    self.bus.cycle_clock(6);
                }

                // TAX
                0xAA => {
                    self.tax();
                    self.bus.cycle_clock(2);
                }

                // TXA
                0x8A => {
                    self.txa();
                    self.bus.cycle_clock(2);
                }

                // TAY
                0xA8 => {
                    self.tay();
                    self.bus.cycle_clock(2);
                }

                // TYA
                0x98 => {
                    self.tya();
                    self.bus.cycle_clock(2);
                }

                // TSX
                0xBA => {
                    self.tsx();
                    self.bus.cycle_clock(2);
                }

                // TXS
                0x9A => {
                    self.txs();
                    self.bus.cycle_clock(2);
                }

                // PHA
                0x48 => {
                    self.pha();
                    self.bus.cycle_clock(3);
                }

                // PLA
                0x68 => {
                    self.pla();
                    self.bus.cycle_clock(4);
                }

                // PHP
                0x08 => {
                    self.php();
                    self.bus.cycle_clock(3);
                }

                // PLP
                0x28 => {
                    self.plp();
                    self.bus.cycle_clock(4);
                }

                // CLC
                0x18 => {
                    self.clc();
                    self.bus.cycle_clock(2);
                }

                // SEC
                0x38 => {
                    self.sec();
                    self.bus.cycle_clock(2);
                }

                // CLI
                0x58 => {
                    self.cli();
                    self.bus.cycle_clock(2);
                }

                // SEI
                0x78 => {
                    self.sei();
                    self.bus.cycle_clock(2);
                }

                // CLV
                0xB8 => {
                    self.clv();
                    self.bus.cycle_clock(2);
                }

                // CLD
                0xD8 => {
                    self.cld();
                    self.bus.cycle_clock(2);
                }

                // SED
                0xF8 => {
                    self.sed();
                    self.bus.cycle_clock(2);
                }

                // NOP
                0xEA => {
                    self.bus.cycle_clock(2);
                }

                // BRK
                0x00 => {
                    self.interupt_brk();
                }

                _ => {}
        }
    }
}
