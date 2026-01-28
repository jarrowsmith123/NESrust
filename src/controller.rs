

pub struct Joypad {
   strobe: bool,
   button_index: u8,
   button_status: u8,
}

impl Joypad {


   pub fn new() -> Self {
       Joypad {
           strobe: false,
           button_index: 0,
           button_status: 0b0000_0000,
       }

   }
   pub fn write(&mut self, data: u8) {
       self.strobe = data & 1 == 1;
       if self.strobe {
           self.button_index = 0
       }
   }

   pub fn read(&mut self) -> u8 {
       if self.button_index > 7 {
           return 1;
       }
       let response = (self.button_status & (1 << self.button_index)) >> self.button_index;
       if !self.strobe && self.button_index <= 7 {
           self.button_index += 1;
       }
       response
   }

   pub fn set_button_pressed_status(&mut self, button: u8, pressed: bool){
        if pressed{
            self.button_status |= button;
        }
        else{
            self.button_status &= !button;
        }
   }
}
