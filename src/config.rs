use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serenity::model::{
    id::{ChannelId, RoleId},
    prelude::GuildId,
};

pub type ServerMap = HashMap<String, ServerType>;
pub type VoiceChannelConfigs = HashMap<RoleId, VoiceChatConfig>;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub discord_token: String,
    pub servers: ServerMap,
    pub vcs: VoiceChannelConfigs,
}

#[derive(Serialize, Deserialize)]
pub enum ServerType {
    Docker(Docker),
    // Custom(CustomServer),
}

impl ServerType {
    pub fn get_guild_ids(&self) -> &HashSet<GuildId> {
        match self {
            ServerType::Docker(docker) => &docker.guild_ids,
            // ServerType::Custom(custom) => &custom.guild_ids,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Docker {
    pub container_name: String,
    pub connect: String,
    pub guild_ids: HashSet<GuildId>,
}

#[derive(Serialize, Deserialize)]
pub struct CustomServer {
    pub connect: String,
    pub start: CommandDefinition,
    pub stop: CommandDefinition,
    pub pause: Option<CommandDefinition>,
    pub unpause: Option<CommandDefinition>,
    pub restart: Option<CommandDefinition>,
    pub status: StatusCommand,
    pub guild_ids: HashSet<GuildId>,
}

#[derive(Serialize, Deserialize)]
pub struct CommandDefinition {
    // command to connect to server
    pub cmd: String,
    // arguments to pass to command
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct StatusCommand {
    // command to get server status
    pub cmd: String,
    // arguments to pass to command
    #[serde(default)]
    pub args: Vec<String>,
    // string that matches when server is running
    pub running_status: String,
    // string that matches when server is paused
    pub paused_status: String,
    // string that matches when server is stopped
    pub stopped_status: String,
    // string that matches when server is starting
    pub pending_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceChatConfig {
    pub voice_channel_ids: Vec<ChannelId>,
    pub text_channel_id: ChannelId,
    pub guild_ids: HashSet<GuildId>,
    pub start_msgs: Vec<String>,
    pub end_msgs: Vec<String>,
}
