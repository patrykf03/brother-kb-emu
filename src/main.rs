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
        
        // Build mapping table based on empirical testing  
        // Format: (mux_a_channel, mux_b_channel)
        // From test data "abcdefghijk" → "n|;4liupeof"
        
        // Empirically verified mappings:
        key_map.insert('n', (1, 2));  // a→n
        key_map.insert('e', (1, 5));  // i→e  
        key_map.insert('f', (2, 5));  // k→f
        key_map.insert('i', (7, 3));  // f→i
        key_map.insert('l', (5, 3));  // e→l
        key_map.insert('o', (1, 0));  // j→o
        key_map.insert('p', (7, 0));  // h→p
        key_map.insert('u', (1, 3));  // g→u
        key_map.insert('4', (0, 7));  // d→4
        key_map.insert(';', (2, 4));  // c→;
        
        // From mux_b pattern analysis:
        // mux_b: 0→X4, 2→X6, 3→X5, 4→X0, 5→X2, 7→X3
        // From mux_a pattern analysis:
        // mux_a: 0→Y1, 1→Y3, 2→Y3 or Y4, 5→Y6, 7→Y5
        
        // Need to fill in rest - inferring from keyboard layout
        // Row X0 (mux_b=4):
        key_map.insert(' ', (5, 4));  // Guessing SPACE
        key_map.insert(',', (6, 4));
        key_map.insert('.', (0, 4));
        key_map.insert('$', (4, 4));
        // ';' already mapped above
        key_map.insert(':', (2, 4));
        key_map.insert('\n', (2, 4)); // RETURN
        key_map.insert('\'', (1, 4));
        key_map.insert('"', (1, 4));
        
        // Row X1 (mux_b=?): need to find
        key_map.insert('/', (6, 2));  
        key_map.insert('?', (6, 2)); 
        key_map.insert('*', (0, 2));  
        key_map.insert('q', (4, 2));  
        key_map.insert('z', (7, 2));  
        key_map.insert('w', (2, 2));  
        key_map.insert('a', (1, 2));  
        
        // Row X2 (mux_b=5):
        key_map.insert('1', (6, 5));  
        key_map.insert('2', (0, 5));  
        // 'e' already mapped (1,5)
        // 'f' already mapped (2,5)
        key_map.insert('r', (2, 5));  
        key_map.insert('g', (1, 5));  
        
        // Row X3 (mux_b=7):
        key_map.insert('3', (6, 7));  
        // '4' already mapped (0,7)
        key_map.insert('t', (4, 7));  
        key_map.insert('h', (7, 7));  
        key_map.insert('y', (2, 7));  
        key_map.insert('j', (1, 7));  
        
        // Row X4 (mux_b=0):
        key_map.insert('7', (0, 0));  
        key_map.insert('8', (4, 0));  
        // 'o' already mapped (1,0)
        key_map.insert('s', (2, 0));  
        // 'p' already mapped (7,0)
        key_map.insert('d', (5, 0));  
        
        // Row X5 (mux_b=3):
        key_map.insert('5', (0, 3));  
        key_map.insert('6', (4, 3));  
        // 'u' already mapped (1,3)
        key_map.insert('k', (2, 3));  
        // 'i' already mapped (7,3)
        // 'l' already mapped (5,3)
        
        // Row X6 (mux_b=2):
        key_map.insert('-', (0, 2));  
        key_map.insert('_', (0, 2));  
        // 'n' already mapped (1,2)
        key_map.insert('x', (2, 2));  
        key_map.insert('m', (1, 2));  
        
        // Row X7 (mux_b=?): need to find
        key_map.insert('9', (0, 6));  
        key_map.insert('0', (4, 6));  
        key_map.insert('v', (7, 6));  
        key_map.insert('c', (2, 6));  
        key_map.insert('b', (1, 6));  
        key_map.insert('\t', (5, 6)); 
        
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
    println!("Testing: 'hello'\n");
    
    keyboard.type_string("hello")?;

    println!("\nDisabling multiplexers...");
    keyboard.enable.set_high();
    println!("Done.");
    
    Ok(())
}
