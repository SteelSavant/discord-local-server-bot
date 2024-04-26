use std::{
    collections::{hash_map::RandomState, HashMap, HashSet},
    str::FromStr,
    time::Duration,
};

use crate::command::SlashCommand;
use config::{ServerMap, VoiceChannelConfigs};
use rand::Rng;

use serenity::{
    async_trait,
    model::{
        id::{ChannelId, RoleId, UserId},
        prelude::{
            interaction::{Interaction, InteractionResponseType},
            GuildId, Ready,
        },
        voice::VoiceState,
    },
    prelude::*,
};
use strum::IntoEnumIterator;

mod command;
mod config;
mod server_commands;

const DELAY: Duration = Duration::from_secs(15);

struct ServerKey;
impl TypeMapKey for ServerKey {
    type Value = ServerMap;
}

struct VoiceChatConfigKey;
impl TypeMapKey for VoiceChatConfigKey {
    type Value = VoiceChannelConfigs;
}

struct VoiceChatStateKey;
impl TypeMapKey for VoiceChatStateKey {
    type Value = HashMap<RoleId, HashSet<UserId>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let data = ctx.data.read().await;
        let servers = data.get::<ServerKey>().unwrap();

        let guild_ids: HashSet<&GuildId, RandomState> =
            HashSet::from_iter(servers.values().flat_map(|server| server.get_guild_ids()));

        for guild_id in guild_ids {
            let servers = servers
                .iter()
                .filter(|s| s.1.get_guild_ids().contains(guild_id))
                .map(|s| s.0)
                .collect::<Vec<_>>();

            let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
                for slash_command in SlashCommand::iter() {
                    commands.create_application_command(|command| {
                        println!("Registering command {:#?}", slash_command);
                        slash_command.register(&servers, command);

                        command
                    });
                }

                commands
            })
            .await;

            match commands {
                Ok(commands) => {
                    println!(
                        "Guild {} registered slash commands: {:#?}",
                        guild_id, commands
                    );
                }
                Err(err) => {
                    println!("Cannot register slash commands: {}", err);
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command_interaction) = interaction {
            println!("Received command interaction: {:#?}", command_interaction);
            let command = SlashCommand::from_str(&command_interaction.data.name);

            match command {
                Ok(command) => {
                    let data = ctx.data.read().await;
                    let servers = data.get::<ServerKey>().unwrap();

                    if let Err(why) = command_interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message.content(command.pending_msg())
                                })
                        })
                        .await
                    {
                        println!("Cannot respond to slash command: {}", why);
                    } else {
                        let content = command
                            .run(
                                servers,
                                command_interaction.guild_id.unwrap_or_default(),
                                &command_interaction.data.options,
                            )
                            .await;

                        if let Err(why) = command_interaction
                            .create_followup_message(&ctx.http, |response| {
                                response.content(content)
                            })
                            .await
                        {
                            println!("Cannot follow-up to slash command: {}", why);
                        }
                    }
                }
                Err(err) => println!("Cannot parse slash command : {}", err),
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        // println!("Got voice state update: {:?} to {:?}", old, new);

        let user = old
            .as_ref()
            .unwrap_or(&new)
            .member
            .as_ref()
            .map(|v| &v.user);

        match user {
            Some(user) => {
                if user.bot {
                    println!("skipping bot user");
                    return;
                }

                let user_id = user.id;

                let old_channel = old.and_then(|v| v.channel_id);
                let new_channel = new.channel_id;

                match (old_channel, new_channel) {
                    (None, Some(new)) => handle_vc(&ctx, new, user_id, true).await,
                    (Some(old), None) => handle_vc(&ctx, old, user_id, false).await,
                    (Some(old_channel), Some(new_channel)) => {
                        let voice_configs = {
                            let data = ctx.data.read().await;
                            let value = data.get::<VoiceChatConfigKey>();

                            value.cloned().unwrap()
                        };

                        let old_roles: HashSet<_> = voice_configs
                            .iter()
                            .filter_map(|v| {
                                if v.1.voice_channel_ids.contains(&old_channel) {
                                    Some(*v.0)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let new_roles: HashSet<_> = voice_configs
                            .iter()
                            .filter_map(|v| {
                                if v.1.voice_channel_ids.contains(&new_channel) {
                                    Some(*v.0)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if new_roles.len() != old_roles.len()
                            || new_roles.intersection(&old_roles).count() != new_roles.len()
                        {
                            println!("swapping user channels");
                            handle_vc(&ctx, new_channel, user_id, true).await;
                            handle_vc(&ctx, old_channel, user_id, false).await;
                        } else {
                            println!("skipping VC update; roles match");
                        }
                    }
                    (None, None) => println!("skipping VC update"),
                }
            }
            None => return,
        }
    }
}

async fn handle_vc(ctx: &Context, channel_id: ChannelId, user_id: UserId, is_add: bool) {
    let voice_configs = {
        let data = ctx.data.read().await;
        let value = data.get::<VoiceChatConfigKey>();

        value.cloned().unwrap()
    };

    let configs = voice_configs
        .into_iter()
        .filter(|(_, v)| v.voice_channel_ids.contains(&channel_id));

    for (id, config) in configs {
        let (old_len, new_len) = {
            let mut data = ctx.data.write().await;
            let channels = data.get_mut::<VoiceChatStateKey>().unwrap();

            let entry = channels.entry(id).or_default();
            let old_len = entry.len();

            if is_add {
                entry.insert(user_id);
            } else {
                entry.remove(&user_id);
            }

            println!("Got entry: {entry:?}");
            let new_len = entry.len();
            (old_len, new_len)
        };

        let condition = if is_add {
            old_len == 0 && new_len > old_len && !config.start_msgs.is_empty()
        } else {
            new_len == 0 && old_len > new_len && !config.end_msgs.is_empty()
        };

        if condition {
            let ctx_data = ctx.data.clone();

            #[cfg(not(debug_assertions))]
            let http = ctx.http.clone();

            println!("starting msg task");

            let _ = tokio::spawn(async move {
                tokio::time::sleep(DELAY).await;
                let mut data = ctx_data.write().await;
                let channels = data.get_mut::<VoiceChatStateKey>().unwrap();

                let entry = channels.entry(id).or_default();

                println!("Got entry after wait: {entry:?}");

                let skip_condition = if is_add {
                    entry.len() <= 0
                } else {
                    entry.len() > 0
                };

                if skip_condition {
                    if is_add {
                        println!("no entries; skipping start msg");
                    } else {
                        println!("found entries; skipping end msg");
                    }

                    return;
                }

                let msgs = if is_add {
                    config.start_msgs
                } else {
                    config.end_msgs
                };

                let msg_index = rand::thread_rng().gen_range(0..msgs.len());
                let content = format!("<@&{}> {}", id.0, msgs[msg_index]);

                #[cfg(debug_assertions)]
                {
                    if is_add {
                        println!("sending start message: {content}");
                    } else {
                        println!("sending end message: {content}");
                    }
                }

                #[cfg(not(debug_assertions))]
                {
                    let res = config
                        .text_channel_id
                        .send_message(&http, |msg| {
                            msg.allowed_mentions(|v| v.roles(vec![id]));

                            msg.content(content);

                            msg
                        })
                        .await;

                    if let Err(err) = res {
                        if is_add {
                            println!("Got err sending start msg: {err}");
                        } else {
                            println!("Got err sending end msg: {err}");
                        }
                    }
                }
            })
            .await;
        }
    }
}

#[tokio::main]
async fn main() {
    let config = std::fs::read_to_string("./assets/config.ron").unwrap();
    let config = ron::from_str::<config::Config>(&config).unwrap();

    // Login with a bot token from the environment
    let token = config.discord_token;
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(token, intents)
        .type_map_insert::<ServerKey>(config.servers)
        .type_map_insert::<VoiceChatConfigKey>(config.vcs)
        .type_map_insert::<VoiceChatStateKey>(HashMap::new())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
