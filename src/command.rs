use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    json::Value,
    model::prelude::{command::*, interaction::application_command::CommandDataOption, GuildId},
};
use strum::{Display, EnumIter, EnumString};

use crate::{
    config::{ServerMap, ServerType},
    server_commands::ServerCommands,
};

#[derive(Debug, Clone, Copy, EnumIter, EnumString, Display)]
pub enum SlashCommand {
    #[strum(serialize = "connect-server")]
    Connect,
    #[strum(serialize = "start-server")]
    Start,
    #[strum(serialize = "stop-server")]
    Stop,
    #[strum(serialize = "restart-server")]
    Restart,
    #[strum(serialize = "pause-server")]
    Pause,
    #[strum(serialize = "unpause-server")]
    Unpause,
    #[strum(serialize = "resume-server")]
    Resume,
    #[strum(serialize = "server-status")]
    Status,
    #[strum(serialize = "list-servers")]
    List,
}

type ServerNames<'a> = [&'a String];

impl SlashCommand {
    const SERVER_OPTION: &'static str = "server";

    pub fn register<'a>(
        &'a self,
        servers: &'_ ServerNames,
        command: &'a mut CreateApplicationCommand,
    ) -> &mut CreateApplicationCommand {
        let name = format!("{}", self);
        let description = self.description();
        command
            .name(name)
            .description(description)
            .kind(CommandType::ChatInput);

        const MAX_OPTIONS: usize = 25;

        let options = self.options(servers);
        assert!(options.len() <= MAX_OPTIONS);

        for option in options {
            command.add_option(option);
        }

        command
    }

    fn description(&self) -> String {
        match self {
            SlashCommand::Connect => "Returns the connect string for the server".to_string(),
            SlashCommand::Start => "Starts the server if it is not already running".to_string(),
            SlashCommand::Stop => "Stops the server if it is running".to_string(),
            SlashCommand::Restart => {
                "Restarts the server if it is running; otherwise starts it".to_string()
            }
            SlashCommand::Pause => "Pauses the server if it is running".to_string(),
            SlashCommand::Unpause => "Unpauses the server if it is paused".to_string(),
            SlashCommand::Resume => {
                "Resumes the server if it is paused, otherwise starts it".to_string()
            }
            SlashCommand::Status => "Returns the status of the server".to_string(),
            SlashCommand::List => "Returns a list of available servers".to_string(),
        }
    }

    fn options(&self, servers: &ServerNames) -> Vec<CreateApplicationCommandOption> {
        if matches!(self, SlashCommand::List) {
            vec![]
        } else {
            let description = match self {
                SlashCommand::Connect => "The server to get the connection string for",
                SlashCommand::Start => "The server to start",
                SlashCommand::Stop => "The server to stop",
                SlashCommand::Restart => "The server to restart",
                SlashCommand::Pause => "The server to pause",
                SlashCommand::Unpause => "The server to unpause",
                SlashCommand::Resume => "The server to resume",
                SlashCommand::Status => "The server to check the status of",
                SlashCommand::List => unreachable!(),
            };
            let mut option = CreateApplicationCommandOption::default();

            option
                .name(Self::SERVER_OPTION)
                .description(description)
                .kind(CommandOptionType::String)
                .required(true);

            const MAX_OPTIONS: usize = 25;

            for server in servers.iter().take(MAX_OPTIONS) {
                option.add_string_choice(server, server);
            }

            vec![option]
        }
    }

    pub async fn run(
        &self,
        servers: &ServerMap,
        guild_id: GuildId,
        options: &[CommandDataOption],
    ) -> String {
        println!("Running command '{:?}' for guild id {}", self, guild_id);

        if matches!(self, SlashCommand::List) {
            {
                let mut res = String::new();

                let servers = servers
                    .iter()
                    .filter(|s| s.1.get_guild_ids().contains(&guild_id))
                    .map(|s| s.0)
                    .collect::<Vec<_>>();

                match servers.len() {
                    0 => res.push_str("No servers available"),
                    1 => {
                        res.push_str("Available server: ");
                        res.push_str(&servers[0]);
                    }
                    _ => {
                        res.push_str("Available servers: ");
                        for name in servers.iter() {
                            res.push_str(&format!("{}, ", name));
                        }
                    }
                }

                res
            }
        } else {
            let server_name = match options.get(0) {
                Some(&CommandDataOption {
                    name: ref cmd_name,
                    value: Some(Value::String(ref server_name)),
                    ..
                }) if *cmd_name == *Self::SERVER_OPTION => server_name.trim(),
                _ => return "No server specified".to_string(),
            };

            let server = servers.get(server_name);
            match server {
                Some(server) => {
                    if server.get_guild_ids().contains(&guild_id) {
                        self.run_with_server(server_name, server).await
                    } else {
                        format!("Server {server_name} not found")
                    }
                }
                None => format!("Server {server_name} not found"),
            }
        }
    }

    async fn run_with_server(&self, server_name: &str, server: &ServerType) -> String {
        match self {
            // Server Management
            SlashCommand::Connect => match server.connect().await {
                Ok(connect_string) => connect_string,
                Err(err) => err.to_string(),
            },
            SlashCommand::Start => match server.start_server() {
                Ok(_) => format!(
                    "Started server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Stop => match server.stop_server() {
                Ok(_) => format!(
                    "Stopped server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Restart => match server.restart_server() {
                Ok(_) => format!(
                    "Restarted server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Pause => match server.pause_server() {
                Ok(_) => format!(
                    "Paused server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Unpause => match server.unpause_server() {
                Ok(_) => format!(
                    "Unpaused server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Resume => match server.resume_server() {
                Ok(_) => format!(
                    "Resumed server {} -- status: {:?}",
                    server_name,
                    server.get_status()
                ),
                Err(err) => err.to_string(),
            },
            SlashCommand::Status => match server.get_status() {
                Ok(status) => format!("Server {} status: {:?}", server_name, status),
                Err(err) => err.to_string(),
            },
            SlashCommand::List => unreachable!(),
            // Role Management
        }
    }

    pub fn pending_msg(&self) -> String {
        match self {
            SlashCommand::Connect => "Getting connect string...".to_string(),
            SlashCommand::Start => "Starting server...".to_string(),
            SlashCommand::Stop => "Stopping server...".to_string(),
            SlashCommand::Restart => "Restarting server...".to_string(),
            SlashCommand::Pause => "Pausing server...".to_string(),
            SlashCommand::Unpause => "Unpausing server...".to_string(),
            SlashCommand::Resume => "Resuming server...".to_string(),
            SlashCommand::Status => "Getting server status...".to_string(),
            SlashCommand::List => "Getting available server list...".to_string(),
        }
    }
}
