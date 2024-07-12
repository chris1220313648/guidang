#!/bin/bash

# Build the project in release mode
cargo build --release
# Start a new terminator window for the second group of commands
terminator -T "Group 2" &

# Wait for a moment to ensure the window is created
sleep 2

# Split the second group of commands in the second window
# xdotool key Ctrl+Shift+O
xdotool type 'mosquitto'
xdotool key Return

sleep 1
xdotool key Ctrl+Shift+O
xdotool type 'serve /root/guidang/config/register'
xdotool key Return

# Start terminator with a custom layout
terminator -T "Group 1" &

# Wait for a moment to ensure the window is created
sleep 2

# Split the first group of commands in the first window
# xdotool key Ctrl+Shift+O
xdotool type './target/release/cloud'
xdotool key Return

sleep 1
xdotool key Ctrl+Shift+O
xdotool type './target/release/filter-server'
xdotool key Return

sleep 1
xdotool key Ctrl+Shift+O
xdotool type './target/release/deno_executor "http://127.0.0.1:8001"'
xdotool key Return



# Start a new terminator window for the second group of commands
terminator -T "Group 3" &

# Wait for a moment to ensure the window is created
sleep 2

# Split the second group of commands in the second window
# xdotool key Ctrl+Shift+O
xdotool type 'python3 ./utils/mapper/switch.py'
xdotool key Return

sleep 1
xdotool key Ctrl+Shift+O
xdotool type 'python3 ./utils/mapper/temp.py'
xdotool key Return
