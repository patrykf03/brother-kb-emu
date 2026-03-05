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

// TEST CONFIGURATION: Change these to test different wiring interpretations
// Try: 0=normal (S0,S1,S2), 1=reversed (S2,S1,S0), 2=swapped S0<->S2 only, etc.
const MUX_A_REVERSE: bool = true;   // Set false for normal, true for reversed
const MUX_B_REVERSE: bool = false;  // Set false for normal, true for reversed

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
        
        // Build mapping table based on mappingtable.md
        // Format: (mux_a_channel, mux_b_channel)
        // Rows = X0-X7 (mux_b), Columns = Y7-Y0 (mux_a, where Y7=7, Y6=6, ..., Y0=0)
        
        // X0 (Mux B = 0): Y7=SHIFT, Y6=SPACE, Y5=' ", Y4=RETURN, Y3=: ;, Y2=$, Y1=., Y0=,
        key_map.insert(' ', (6, 0));  // SPACE at Y6
        key_map.insert(',', (0, 0));  // , at Y0
        key_map.insert('.', (1, 0));  // . at Y1
        key_map.insert('$', (2, 0));  // $ at Y2
        key_map.insert(';', (3, 0));  // : ; at Y3
        key_map.insert(':', (3, 0));  // : ; at Y3
        key_map.insert('\n', (4, 0)); // RETURN at Y4
        key_map.insert('\'', (5, 0)); // ' " at Y5
        key_map.insert('"', (5, 0));  // ' " at Y5
        
        // X1 (Mux B = 1): Y7=CAPS, Y6=RETURN, Y5=A, Y4=W, Y3=Z, Y2=Q, Y1=*, Y0=/ ?
        key_map.insert('/', (0, 1));  // / ? at Y0
        key_map.insert('?', (0, 1));  // / ? at Y0
        key_map.insert('*', (1, 1));  // * at Y1
        key_map.insert('q', (2, 1));  // Q at Y2
        key_map.insert('z', (3, 1));  // Z at Y3
        key_map.insert('w', (4, 1));  // W at Y4
        key_map.insert('a', (5, 1));  // A at Y5
        
        // X2 (Mux B = 2): Y7=P INS, Y6=INDEX, Y5=G, Y4=R, Y3=F, Y2=E, Y1=2, Y0=1
        key_map.insert('1', (0, 2));  // 1 at Y0
        key_map.insert('2', (1, 2));  // 2 at Y1
        key_map.insert('e', (2, 2));  // E at Y2
        key_map.insert('f', (3, 2));  // F at Y3
        key_map.insert('r', (4, 2));  // R at Y4
        key_map.insert('g', (5, 2));  // G at Y5
        
        // X3 (Mux B = 3): Y7=L IND, Y6=RELOC, Y5=J, Y4=Y, Y3=H, Y2=T, Y1=4, Y0=3
        key_map.insert('3', (0, 3));  // 3 at Y0
        key_map.insert('4', (1, 3));  // 4 at Y1
        key_map.insert('t', (2, 3));  // T at Y2
        key_map.insert('h', (3, 3));  // H at Y3
        key_map.insert('y', (4, 3));  // Y at Y4
        key_map.insert('j', (5, 3));  // J at Y5
        
        // X4 (Mux B = 4): Y7=CODE, Y6=D, Y5=P, Y4=S, Y3=O, Y2=8, Y1=7, Y0=Unused
        key_map.insert('7', (1, 4));  // 7 at Y1
        key_map.insert('8', (2, 4));  // 8 at Y2
        key_map.insert('o', (3, 4));  // O at Y3
        key_map.insert('s', (4, 4));  // S at Y4
        key_map.insert('p', (5, 4));  // P at Y5
        key_map.insert('d', (6, 4));  // D at Y6
        
        // X5 (Mux B = 5): Y7=RETURN, Y6=L, Y5=I, Y4=K, Y3=U, Y2=6, Y1=5, Y0=Unused
        key_map.insert('5', (1, 5));  // 5 at Y1
        key_map.insert('6', (2, 5));  // 6 at Y2
        key_map.insert('u', (3, 5));  // U at Y3
        key_map.insert('k', (4, 5));  // K at Y4
        key_map.insert('i', (5, 5));  // I at Y5
        key_map.insert('l', (6, 5));  // L at Y6
        
        // X6 (Mux B = 6): Y7=Cancel, Y6=ALT, Y5=M, Y4=X, Y3=N, Y2=1/4, Y1=- _, Y0=Unused
        key_map.insert('-', (1, 6));  // - _ at Y1
        key_map.insert('_', (1, 6));  // - _ at Y1
        key_map.insert('n', (3, 6));  // N at Y3
        key_map.insert('x', (4, 6));  // X at Y4
        key_map.insert('m', (5, 6));  // M at Y5
        
        // X7 (Mux B = 7): Y7=WORD OUT, Y6=TAB, Y5=B, Y4=C, Y3=V, Y2=0, Y1=9, Y0=Unused
        key_map.insert('9', (1, 7));  // 9 at Y1
        key_map.insert('0', (2, 7));  // 0 at Y2
        key_map.insert('v', (3, 7));  // V at Y3
        key_map.insert('c', (4, 7));  // C at Y4
        key_map.insert('b', (5, 7));  // B at Y5
        key_map.insert('\t', (6, 7)); // TAB at Y6
        
        Ok(Keyboard {
            mux_a: if MUX_A_REVERSE {
                Multiplexer::new(gpio, MUX_A_S2, MUX_A_S1, MUX_A_S0)?
            } else {
                Multiplexer::new(gpio, MUX_A_S0, MUX_A_S1, MUX_A_S2)?
            },
            mux_b: if MUX_B_REVERSE {
                Multiplexer::new(gpio, MUX_B_S2, MUX_B_S1, MUX_B_S0)?
            } else {
                Multiplexer::new(gpio, MUX_B_S0, MUX_B_S1, MUX_B_S2)?
            },
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
    println!("===========================================");
    println!("Current configuration:");
    println!("  MUX_A_REVERSE = {}", MUX_A_REVERSE);
    println!("  MUX_B_REVERSE = {}", MUX_B_REVERSE);
    println!("===========================================\n");
    println!("Testing: 'hello'\n");
    
    keyboard.type_string("hello")?;

    println!("\n===========================================");
    println!("If output is still wrong, edit main.rs and change:");
    println!("  const MUX_A_REVERSE = true/false");
    println!("  const MUX_B_REVERSE = true/false");
    println!("Try all 4 combinations: (false,false), (true,false), (false,true), (true,true)");
    println!("===========================================");
    println!("\nDisabling multiplexers...");
    keyboard.enable.set_high();
    println!("Done.");
    
    Ok(())
}
