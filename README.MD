# Server-Bot

This bot has two primary functionalities:

- Allow enable/disabling docker containers hosting game servers via discord
- Allow notifying a text channel that other members of the server are in a voice channel

One day these functionalities will be combined. One day...

## Configuration

A template is available at `/templates/config.ron` in this repository.

Your config file needs to be stored at `./assets/config.ron`, in the same directory
as the bot executable.

Several peices of information are required to configure the bot

- a discord bot token
- guild (server), channel, and role ids for the appropriate elements in your discord server

### Getting a bot token

Create a discord application. The [official docs](https://discord.com/developers/docs/quick-start/getting-started) are a good place to get started.

Under the "Bot" section of the application, find your token, and paste it into your config file.

Under the OAuth2 section, select

- scopes
  - bot
  - applications.commands
- bot permissions
  - Send Messages
  - Mention Everyone
  - Use Slash Commands
  - Use Voice Activity

Copy the generated url into a browser, and use it to add the bot to your server.

### Getting Discord ids

Right click on the channel/role/etc. in discord and select "Copy X id".

## Usage

Be sure to compile in release mode (`cargo build --release`); debug is configured to not actually ping message channels.

This document assumes the bot will run on linux (and that you have a basic knowledge of the same). Configure a `systemd` service to run the bot on system boot.

There is a simple build script in the `/scripts` folder that builds and copies the bot executable into the `/bin` folder, but you'll still need to configure the `systemd` service yourself.
