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
    
    /// Set the channel (0-7) for this multiplexer
    /// The enable pin is disabled during transition and re-enabled after
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
        
        // Build mapping table based on mappingtable.md
        // Format: (mux_a_channel, mux_b_channel) where channels are 0-indexed
        // Row index = mux_b channel (0-7), Column index = mux_a channel (0-7)
        
        // Row 1 (Mux B = 0)
        key_map.insert(' ', (0, 0));  // spacebar
        key_map.insert(';', (5, 0));
        key_map.insert(',', (6, 0));
        key_map.insert('.', (7, 0));
        
        // Row 2 (Mux B = 1)
        key_map.insert('\n', (0, 1)); // enter
        key_map.insert('w', (2, 1));
        key_map.insert('a', (3, 1));
        key_map.insert('q', (4, 1));
        key_map.insert('z', (5, 1));
        key_map.insert('+', (7, 1));
        
        // Row 3 (Mux B = 2)
        key_map.insert('r', (2, 2));
        key_map.insert('g', (3, 2));
        key_map.insert('e', (4, 2));
        key_map.insert('f', (5, 2));
        key_map.insert('1', (6, 2));
        key_map.insert('2', (7, 2));
        
        // Row 4 (Mux B = 3)
        key_map.insert('y', (2, 3));
        key_map.insert('j', (3, 3));
        key_map.insert('t', (4, 3));
        key_map.insert('h', (5, 3));
        key_map.insert('3', (6, 3));
        key_map.insert('4', (7, 3));
        
        // Row 5 (Mux B = 4)
        key_map.insert('p', (2, 4));
        key_map.insert('d', (3, 4));
        key_map.insert('o', (4, 4));
        key_map.insert('s', (5, 4));
        key_map.insert('7', (6, 4));
        key_map.insert('8', (7, 4));
        
        // Row 6 (Mux B = 5)
        key_map.insert('i', (2, 5));
        key_map.insert('l', (3, 5));
        key_map.insert('u', (4, 5));
        key_map.insert('k', (5, 5));
        key_map.insert('5', (6, 5));
        key_map.insert('6', (7, 5));
        
        // Row 7 (Mux B = 6)
        key_map.insert('m', (2, 6));
        key_map.insert('n', (4, 6));
        key_map.insert('x', (5, 6));
        key_map.insert('?', (6, 6));
        
        // Row 8 (Mux B = 7)
        key_map.insert('b', (2, 7));
        key_map.insert('v', (4, 7));
        key_map.insert('c', (5, 7));
        key_map.insert('9', (6, 7));
        key_map.insert('0', (7, 7));
        
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
        
        // Hold for 10ms
        thread::sleep(Duration::from_millis(50));
        
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
            thread::sleep(Duration::from_millis(50));
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
    println!("Ready to type text to the typewriter.");
    println!("---");
    
    // Example: Type a test string
    let test_text = "hello world 123";
    println!("Typing: \"{}\"", test_text);
    
    keyboard.type_string(test_text)?;
    
    println!("\nTyping complete!");
    println!("Disabling multiplexers...");
    keyboard.enable.set_high();
    println!("Done.");
    
    Ok(())
}
