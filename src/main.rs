use rppal::gpio::{Gpio, OutputPin};
use std::collections::HashMap;
use std::time::Duration;
use std::thread;
use std::io::{self, BufRead};

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
        
        // Confirmed working letters:
        key_map.insert('n', (1, 2));  
        key_map.insert('e', (1, 5));  
        key_map.insert('f', (2, 5));  
        key_map.insert('i', (7, 3));  
        key_map.insert('o', (1, 0));  
        key_map.insert('p', (7, 0));  
        key_map.insert('u', (1, 3));  
        key_map.insert('y', (7, 7));  
        key_map.insert('z', (2, 6));  
        key_map.insert('q', (1, 6));  
        key_map.insert('x', (2, 2));  
        key_map.insert('t', (1, 7));  
        key_map.insert('w', (7, 6));  
        key_map.insert('m', (7, 2));  
        
        // Fixed from latest test:
        key_map.insert('h', (2, 7));  // c→h, so h is at (2,7)
        key_map.insert('s', (2, 0));  // j→s, so s is at (2,0)
        key_map.insert('2', (0, 5));  // l→2, so 2 is at (0,5)
        
        // Numbers:
        key_map.insert('1', (5, 3));  
        key_map.insert('3', (3, 7));  
        key_map.insert('4', (0, 7));  
        
        // Still need to find correct positions for: a, b, c, d, g, j, k, l, r, v
        // Testing with unused mux positions:
        key_map.insert('a', (6, 2));  // Trying different position
        key_map.insert('b', (1, 4));  // Currently produces |, needs fix
        key_map.insert('c', (4, 7));  // Trying different position
        key_map.insert('d', (5, 0));  // Guess
        key_map.insert('g', (4, 3));  // Trying different position 
        key_map.insert('j', (0, 0));  // Trying different position
        key_map.insert('k', (2, 3));  // Guess
        key_map.insert('l', (0, 3));  // Trying different position
        key_map.insert('r', (4, 5));  // Trying different position
        key_map.insert('v', (7, 4));  // Currently makes fraction, needs fix
        
        // Punctuation and special
        key_map.insert(' ', (6, 0));  // Trying different position
        key_map.insert(',', (6, 1));
        key_map.insert('.', (0, 4));
        key_map.insert('$', (4, 4));
        key_map.insert(';', (2, 4));
        key_map.insert(':', (2, 4));
        key_map.insert('\n', (2, 1));
        key_map.insert('\'', (1, 1));
        key_map.insert('"', (1, 1));
        key_map.insert('/', (6, 2));  
        key_map.insert('?', (6, 2)); 
        key_map.insert('*', (0, 2));  
        key_map.insert('-', (4, 6));  
        key_map.insert('_', (4, 6));  
        key_map.insert('\t', (0, 4));
        
        // More numbers
        key_map.insert('0', (5, 4));
        key_map.insert('5', (4, 5));  
        key_map.insert('6', (5, 5));  
        key_map.insert('7', (4, 0));  
        key_map.insert('8', (5, 7));  
        key_map.insert('9', (4, 4)); 
        
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
    
    /// Type a string with Enter confirmation between each character
    fn type_string_interactive(&mut self, text: &str) -> Result<(), String> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        
        for ch in text.chars() {
            // Convert to lowercase for simplicity
            let ch_lower = ch.to_ascii_lowercase();
            
            // Skip unmapped characters with a warning
            if !self.key_map.contains_key(&ch_lower) {
                println!("Warning: Skipping unmapped character '{}'", ch);
                continue;
            }
            
            println!("\n--- Ready to type '{}' (Press Enter) ---", ch);
            let mut line = String::new();
            handle.read_line(&mut line).map_err(|e| e.to_string())?;
            
            self.press_key(ch_lower)?;
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
    println!("Testing: 'the quick brown fox jumps over the lazy dog'\\n");
    
    keyboard.type_string_interactive("the quick brown fox jumps over the lazy dog")?;

    println!("\nDisabling multiplexers...");
    keyboard.enable.set_high();
    println!("Done.");
    
    Ok(())
}
