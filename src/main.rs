use rppal::gpio::{Gpio, OutputPin};
use std::process;

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
        if (channel >> 0) & 1 == 1 {
            self.s0.set_high();
        } else {
            self.s0.set_low();
        }
        
        // Set S1 (Bit 1)
        if (channel >> 1) & 1 == 1 {
            self.s1.set_high();
        } else {
            self.s1.set_low();
        }
        
        // Set S2 (Bit 2)
        if (channel >> 2) & 1 == 1 {
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

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up Ctrl+C handler
    ctrlc::set_handler(|| {
        println!("\nReceived Ctrl+C, exiting...");
        process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");
    
    println!("Initializing GPIO pins...");
    
    // Initialize GPIO
    let gpio = match Gpio::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error initializing GPIO: {}", e);
            process::exit(1);
        }
    };
    
    // Initialize enable pin and multiplexers
    let mut enable = gpio.get(ENABLE_PIN)?.into_output();
    let mut mux_a = Multiplexer::new(&gpio, MUX_A_S0, MUX_A_S1, MUX_A_S2)?;
    let mut mux_b = Multiplexer::new(&gpio, MUX_B_S0, MUX_B_S1, MUX_B_S2)?;
    
    // Enable the multiplexers (active LOW)
    enable.set_low();
    
    println!("Initialization complete. Starting channel sequence.");
    println!("Cycling through all combinations with 2s delay. Press Ctrl+C to exit.");
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Iterate through all 64 combinations (8x8)
    // Outer loop: Mux B, Inner loop: Mux A
    for b_ch in 0..=7 {
        for a_ch in 0..=7 {
            clear_screen();
            println!("--- Multiplexer Control ---");
            println!("Setting Mux A: Channel {} | Mux B: Channel {}", a_ch, b_ch);
            println!("---------------------------");
            println!("Mux A Pins (S0,S1,S2): {}, {}, {}", MUX_A_S0, MUX_A_S1, MUX_A_S2);
            println!("Mux B Pins (S0,S1,S2): {}, {}, {}", MUX_B_S0, MUX_B_S1, MUX_B_S2);
            println!("Enable Pin: {} (State: LOW/ON)", ENABLE_PIN);
            
            // Set both channels
            mux_a.set_channel(a_ch, &mut enable);
            mux_b.set_channel(b_ch, &mut enable);
            
            // Wait 2 seconds before next combination
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    
    println!("\nAll combinations have been cycled through.");
    
    // Cleanup: disable multiplexers
    println!("Disabling multiplexers...");
    enable.set_high();
    println!("Done.");
    Ok(())
}
