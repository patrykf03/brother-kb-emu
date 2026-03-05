#!/bin/bash

# A script to cycle through all 64 combinations of two 8-channel
# multiplexers connected to a Raspberry Pi's GPIO header.

# --- Pin Definitions based on the schematic ---
# Enable pin (active LOW) for both multiplexers
ENABLE_PIN=6

# Multiplexer A Pins (S0, S1, S2)
MUX_A_S0=17 # Sel0
MUX_A_S1=27 # Sel1
MUX_A_S2=22 # Sel2

# Multiplexer B Pins (S0, S1, S2)
MUX_B_S0=19 # Sel3
MUX_B_S1=26 # Sel4
MUX_B_S2=21 # Sel5

# --- Cleanup Function ---
# This function is called when the script exits to ensure pins are left
# in a safe state (multiplexers disabled).
cleanup() {
    echo -e "\nExiting. Disabling multiplexers..."
    # Drive Enable pin HIGH to disable the multiplexers as per truth table (E=H)
    pinctrl $ENABLE_PIN op dh
    # Optional: set select pins back to inputs
    pinctrl $MUX_A_S0,$MUX_A_S1,$MUX_A_S2,$MUX_B_S0,$MUX_B_S1,$MUX_B_S2 ip
    echo "Done."
}

# Trap the EXIT signal to run the cleanup function
trap cleanup EXIT

# --- Helper Function to set a multiplexer's channel ---
# Usage: set_channel <channel_number> <S0_pin> <S1_pin> <S2_pin>
# Channel number should be 0-7.
set_channel() {
    local channel=$1
    local s0_pin=$2
    local s1_pin=$3
    local s2_pin=$4

    pinctrl $ENABLE_PIN op dh

    # Set S0 (Bit 0)
    if (( (channel >> 0) & 1 )); then
        pinctrl $s0_pin op dh # Drive High
    else
        pinctrl $s0_pin op dl # Drive Low
    fi

    # Set S1 (Bit 1)
    if (( (channel >> 1) & 1 )); then
        pinctrl $s1_pin op dh # Drive High
    else
        pinctrl $s1_pin op dl # Drive Low
    fi

    # Set S2 (Bit 2)
    if (( (channel >> 2) & 1 )); then
        pinctrl $s2_pin op dh # Drive High
    else
        pinctrl $s2_pin op dl # Drive Low
    fi

    pinctrl $ENABLE_PIN op dl
}

# --- Main Script Logic ---

echo "Initializing GPIO pins..."
# Enable the multiplexers by driving the Enable pin LOW
# Note: The 'dl' in 'op dl' means 'drive low'. This corresponds to E=L in the table.
pinctrl $ENABLE_PIN op dl

echo "Initialization complete. Starting channel sequence."
echo "Press any key to step through combinations. Press Ctrl+C to exit."
sleep 1

# The prompt asks to go from 1-1 to 8-8 and then change on keypress,
# with an example of "1-1 ... 2-1...". This suggests iterating Mux B
# in the outer loop and Mux A in the inner loop.
# We will use 1-8 for user display and 0-7 for the logic.
for b_ch in {1..8}; do
    for a_ch in {1..8}; do
        clear
        echo "--- Multiplexer Control ---"
        echo "Setting Mux A: Channel $a_ch | Mux B: Channel $b_ch"
        echo "---------------------------"
        echo "Mux A Pins (S0,S1,S2): $MUX_A_S0, $MUX_A_S1, $MUX_A_S2"
        echo "Mux B Pins (S0,S1,S2): $MUX_B_S0, $MUX_B_S1, $MUX_B_S2"
        echo "Enable Pin: $ENABLE_PIN (State: LOW/ON)"
        
        # Set the channels. We subtract 1 because loops are 1-8 but logic is 0-7.
        set_channel $((a_ch - 1)) $MUX_A_S0 $MUX_A_S1 $MUX_A_S2
        set_channel $((b_ch - 1)) $MUX_B_S0 $MUX_B_S1 $MUX_B_S2
        
        # Wait for a single key press (-n 1), silently (-s)
        read -n 1 -s -r -p "Press any key for the next combination..."
    done
done

echo -e "\nAll combinations have been cycled through."
# The cleanup function will be called automatically on exit.
