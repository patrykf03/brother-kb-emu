fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing GPIO pins...");
    
    // Initialize GPIO
    let gpio = Gpio::new()?;
    
    // Initialize enable pin and multiplexers
    let mut enable = gpio.get(ENABLE_PIN)?.into_output();
    let mut mux_a = Multiplexer::new(&gpio, MUX_A_S0, MUX_A_S1, MUX_A_S2)?;
    let mut mux_b = Multiplexer::new(&gpio, MUX_B_S0, MUX_B_S1, MUX_B_S2)?;
    
    // Enable the multiplexers (active LOW)
    enable.set_low();
    
    println!("Initialization complete. Starting channel sequence.");
    println!("Cycling through all combinations with 2s delay.");
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Iterate through all 64 combinations (8x8)
    // Outer loop: Mux B, Inner loop: Mux A
    for b_ch in 0..=7 {
        for a_ch in 0..=7 {
            println!("--- Multiplexer Control ---");
            println!("Setting Mux A: Channel {} | Mux B: Channel {}", a_ch, b_ch);
            println!("---------------------------");
            println!("Mux A Pins (S0,S1,S2): {}, {}, {}", MUX_A_S0, MUX_A_S1, MUX_A_S2);
            println!("Mux B Pins (S0,S1,S2): {}, {}, {}", MUX_B_S0, MUX_B_S1, MUX_B_S2);
            println!("Enable Pin: {} (State: LOW/ON)", ENABLE_PIN);
            
            // Set both channels
            enable.set_high(); // Disable before changing channels
            mux_a.set_channel(a_ch, &mut enable);
            mux_b.set_channel(b_ch, &mut enable);
            enable.set_low(); // Re-enable after setting channels
            
            // Wait 1 second before next combination
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    
    println!("\nAll combinations have been cycled through.");
    
    // Cleanup: disable multiplexers
    println!("Disabling multiplexers...");
    enable.set_high();
    println!("Done.");
    Ok(())
}
