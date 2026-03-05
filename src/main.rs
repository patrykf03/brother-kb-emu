use rppal::gpio::{Gpio, OutputPin};
use std::collections::HashMap;
use std::time::Duration;
use std::thread;

// Pin Definitions based on the schematic
const ENABLE_PIN: u8 = 6;

// Multiplexer A Pins (S0, S1, S2)
const MUX_A_S0: u8 = 17; // Sel0
const MUX_A_S1: u8 = 27; // Sel1
const MUX_A_S2: u8 = 22; // Sel2

// Multiplexer B Pins (S0, S1, S2)
const MUX_B_S0: u8 = 19; // Sel3
const MUX_B_S1: u8 = 26; // Sel4
const MUX_B_S2: u8 = 21; // Sel5

/// Represents a single 8-channel multiplexer with 3 select pins
struct Multiplexer {
    s0: OutputPin,
    s1: OutputPin,
    s2: OutputPin,
}

impl Multiplexer {
    fn new(gpio: &Gpio, s0_pin: u8, s1_pin: u8, s2_pin: u8) -> Result<Self, rppal::gpio::Error> {
        Ok(Multiplexer {
            s0: gpio.get(s0_pin)?.into_output(),
            s1: gpio.get(s1_pin)?.into_output(),
            s2: gpio.get(s2_pin)?.into_output(),
        })
    }
    
    /// Set the channel (0-7) for this multiplexer - NORMAL bit order
    fn set_channel(&mut self, channel: u8, enable: &mut OutputPin) {
        // save current enable state
        let was_enabled = enable.is_set_low();
        // Disable the multiplexer (active LOW)
        enable.set_high();
        
        // Set S0 (Bit 0)
        if channel & (1 << 0) != 0 {
            self.s0.set_high();
        } else {
            self.s0.set_low();
        }
        
        // Set S1 (Bit 1)
        if channel & (1 << 1) != 0 {
            self.s1.set_high();
        } else {
            self.s1.set_low();
        }
        
        // Set S2 (Bit 2)
        if channel & (1 << 2) != 0 {
            self.s2.set_high();
        } else {
            self.s2.set_low();
        }
        
        // Re-enable the multiplexer if it was previously enabled
        if was_enabled {
            enable.set_low();
        }
    }
}

/// Represents the keyboard with mapping from characters to (mux_a_channel, mux_b_channel)
struct Keyboard {
    mux_a: Multiplexer,
    mux_b: Multiplexer,
    enable: OutputPin,
    key_map: HashMap<char, (u8, u8)>,
}

impl Keyboard {
    fn new(gpio: &Gpio) -> Result<Self, rppal::gpio::Error> {
        let mut key_map = HashMap::new();
        
        // Build mapping table based on schematic
        // Format: (mux_a_channel, mux_b_channel)
        // Mux channels map to matrix positions via scrambled connections:
        // Mux A: ch0→Y6, ch1→Y5, ch2→Y4, ch3→Y7, ch4→Y1, ch5→Y2, ch6→Y0, ch7→Y3
        // Mux B: ch0→X3, ch1→X0, ch2→X1, ch3→X2, ch4→X7, ch5→X5, ch6→X6, ch7→X4
        
        // X0 (Mux B = 1): Y6=SPACE, Y0=, Y1=. Y2=$ Y3=: ; Y4=RETURN Y5=' "
        key_map.insert(' ', (0, 1));  // SPACE at Y6 → mux_a=0
        key_map.insert(',', (6, 1));  // , at Y0 → mux_a=6
        key_map.insert('.', (4, 1));  // . at Y1 → mux_a=4
        key_map.insert('$', (5, 1));  // $ at Y2 → mux_a=5
        key_map.insert(';', (7, 1));  // : ; at Y3 → mux_a=7
        key_map.insert(':', (7, 1));  // : ; at Y3 → mux_a=7
        key_map.insert('\n', (2, 1)); // RETURN at Y4 → mux_a=2
        key_map.insert('\'', (1, 1)); // ' " at Y5 → mux_a=1
        key_map.insert('"', (1, 1));  // ' " at Y5 → mux_a=1
        
        // X1 (Mux B = 2): Y0=/ ? Y1=* Y2=Q Y3=Z Y4=W Y5=A
        key_map.insert('/', (6, 2));  // / ? at Y0 → mux_a=6
        key_map.insert('?', (6, 2));  // / ? at Y0 → mux_a=6
        key_map.insert('*', (4, 2));  // * at Y1 → mux_a=4
        key_map.insert('q', (5, 2));  // Q at Y2 → mux_a=5
        key_map.insert('z', (7, 2));  // Z at Y3 → mux_a=7
        key_map.insert('w', (2, 2));  // W at Y4 → mux_a=2
        key_map.insert('a', (1, 2));  // A at Y5 → mux_a=1
        
        // X2 (Mux B = 3): Y0=1 Y1=2 Y2=E Y3=F Y4=R Y5=G
        key_map.insert('1', (6, 3));  // 1 at Y0 → mux_a=6
        key_map.insert('2', (4, 3));  // 2 at Y1 → mux_a=4
        key_map.insert('e', (5, 3));  // E at Y2 → mux_a=5
        key_map.insert('f', (7, 3));  // F at Y3 → mux_a=7
        key_map.insert('r', (2, 3));  // R at Y4 → mux_a=2
        key_map.insert('g', (1, 3));  // G at Y5 → mux_a=1
        
        // X3 (Mux B = 0): Y0=3 Y1=4 Y2=T Y3=H Y4=Y Y5=J
        key_map.insert('3', (6, 0));  // 3 at Y0 → mux_a=6
        key_map.insert('4', (4, 0));  // 4 at Y1 → mux_a=4
        key_map.insert('t', (5, 0));  // T at Y2 → mux_a=5
        key_map.insert('h', (7, 0));  // H at Y3 → mux_a=7
        key_map.insert('y', (2, 0));  // Y at Y4 → mux_a=2
        key_map.insert('j', (1, 0));  // J at Y5 → mux_a=1
        
        // X4 (Mux B = 7): Y1=7 Y2=8 Y3=O Y4=S Y5=P Y6=D
        key_map.insert('7', (4, 7));  // 7 at Y1 → mux_a=4
        key_map.insert('8', (5, 7));  // 8 at Y2 → mux_a=5
        key_map.insert('o', (7, 7));  // O at Y3 → mux_a=7
        key_map.insert('s', (2, 7));  // S at Y4 → mux_a=2
        key_map.insert('p', (1, 7));  // P at Y5 → mux_a=1
        key_map.insert('d', (0, 7));  // D at Y6 → mux_a=0
        
        // X5 (Mux B = 5): Y1=5 Y2=6 Y3=U Y4=K Y5=I Y6=L
        key_map.insert('5', (4, 5));  // 5 at Y1 → mux_a=4
        key_map.insert('6', (5, 5));  // 6 at Y2 → mux_a=5
        key_map.insert('u', (7, 5));  // U at Y3 → mux_a=7
        key_map.insert('k', (2, 5));  // K at Y4 → mux_a=2
        key_map.insert('i', (1, 5));  // I at Y5 → mux_a=1
        key_map.insert('l', (0, 5));  // L at Y6 → mux_a=0
        
        // X6 (Mux B = 6): Y1=- _ Y3=N Y4=X Y5=M
        key_map.insert('-', (4, 6));  // - _ at Y1 → mux_a=4
        key_map.insert('_', (4, 6));  // - _ at Y1 → mux_a=4
        key_map.insert('n', (7, 6));  // N at Y3 → mux_a=7
        key_map.insert('x', (2, 6));  // X at Y4 → mux_a=2
        key_map.insert('m', (1, 6));  // M at Y5 → mux_a=1
        
        // X7 (Mux B = 4): Y1=9 Y2=0 Y3=V Y4=C Y5=B Y6=TAB
        key_map.insert('9', (4, 4));  // 9 at Y1 → mux_a=4
        key_map.insert('0', (5, 4));  // 0 at Y2 → mux_a=5
        key_map.insert('v', (7, 4));  // V at Y3 → mux_a=7
        key_map.insert('c', (2, 4));  // C at Y4 → mux_a=2
        key_map.insert('b', (1, 4));  // B at Y5 → mux_a=1
        key_map.insert('\t', (0, 4)); // TAB at Y6 → mux_a=0
        
        Ok(Keyboard {
            mux_a: Multiplexer::new(gpio, MUX_A_S0, MUX_A_S1, MUX_A_S2)?,
            mux_b: Multiplexer::new(gpio, MUX_B_S0, MUX_B_S1, MUX_B_S2)?,
            enable: gpio.get(ENABLE_PIN)?.into_output(),
            key_map,
        })
    }
    
    /// Press a single key by setting the appropriate mux channels and holding for 10ms
    fn press_key(&mut self, ch: char) -> Result<(), String> {
        // Look up the character in the key map
        let (mux_a_ch, mux_b_ch) = self.key_map.get(&ch)
            .ok_or_else(|| format!("Character '{}' not found in key map", ch))?;
        
        println!("Pressing '{}' → Mux A: {}, Mux B: {}", ch, mux_a_ch, mux_b_ch);
        
        // Disable multiplexers before changing channels
        self.enable.set_high();
        
        // Set both multiplexer channels
        self.mux_a.set_channel(*mux_a_ch, &mut self.enable);
        self.mux_b.set_channel(*mux_b_ch, &mut self.enable);
        
        // Enable the multiplexers to "press" the key
        self.enable.set_low();
        
        // Hold for 100ms
        thread::sleep(Duration::from_millis(100));
        
        // Release the key
        self.enable.set_high();
        
        Ok(())
    }
    
    /// Type a string by pressing each character in sequence
    fn type_string(&mut self, text: &str) -> Result<(), String> {
        for ch in text.chars() {
            // Convert to lowercase for simplicity
            let ch_lower = ch.to_ascii_lowercase();
            
            // Skip unmapped characters with a warning
            if !self.key_map.contains_key(&ch_lower) {
                println!("Warning: Skipping unmapped character '{}'", ch);
                continue;
            }
            
            self.press_key(ch_lower)?;
            
            // Small delay between keypresses for reliability
            thread::sleep(Duration::from_millis(500));
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing Brother Keyboard Emulator...");
    
    // Initialize GPIO
    let gpio = Gpio::new()?;
    
    // Initialize keyboard
    let mut keyboard = Keyboard::new(&gpio)?;
    
    // Start with multiplexers disabled
    keyboard.enable.set_high();
    
    println!("Initialization complete.");
    println!("Testing: 'abcdefghijk'\n");
    
    keyboard.type_string("abcdefghijk")?;

    println!("\nDisabling multiplexers...");
    keyboard.enable.set_high();
    println!("Done.");
    
    Ok(())
}
