#!/bin/bash

cargo build --release

sudo systemctl stop simple-dedicated-server-bot.service

# Create the required directories if they don't exist
mkdir -p /bin/simple-dedicated-server-bot/assets 

# Copy the executable to /bin/simple-dedicated-server-bot/
sudo cp ./target/release/simple-dedicated-server-bot /bin/simple-dedicated-server-bot/simple-dedicated-server-bot

# Copy the assets folder to /bin/simple-dedicated-server-bot/
sudo cp -r ./assets /bin/simple-dedicated-server-bot/

sudo systemctl start simple-dedicated-server-bot.service