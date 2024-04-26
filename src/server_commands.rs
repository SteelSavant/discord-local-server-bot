use serenity::async_trait;

use crate::{
    command::SlashCommand,
    config::{Docker, ServerType},
};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    process,
};

#[derive(Debug)]
pub enum ServerError {
    StatusError(ServerStatus, SlashCommand),
    CommandFailed(SlashCommand, String),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::StatusError(status, command) => write!(
                f,
                "Cannot perform command {} while in state : {:?}",
                command, status
            ),
            ServerError::CommandFailed(cmd, err) => write!(f, "Command {} failed: {}", cmd, err),
        }
    }
}

impl Error for ServerError {}

#[derive(Debug)]
pub enum ServerStatus {
    Running,
    Stopped,
    Paused,

    Unknown(String),
}

#[async_trait]
pub trait ServerCommands {
    /// Returns the connect string for the server
    async fn connect(&self) -> Result<String, ServerError>;

    /// Starts the server if it is not already running
    fn start_server(&self) -> Result<(), ServerError>;

    /// Stops the server if it is running
    fn stop_server(&self) -> Result<(), ServerError>;

    /// Restarts the server if it is running; otherwise starts it
    fn restart_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Stopped) => self.start_server(),
            Ok(_) => self.stop_server().and_then(|_| self.start_server()),
            Err(e) => Err(e),
        }
    }

    /// Pauses the server if it is running
    fn pause_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Running) => self.stop_server(),
            Ok(status) => Err(ServerError::StatusError(status, SlashCommand::Pause)),
            Err(e) => Err(e),
        }
    }

    /// Unpauses the server if it is paused
    fn unpause_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Paused | ServerStatus::Stopped) => self.start_server(),
            Ok(status) => Err(ServerError::StatusError(status, SlashCommand::Unpause)),
            Err(e) => Err(e),
        }
    }

    /// Resumes the server if it is paused, otherwise starts it
    fn resume_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Paused) => self.unpause_server(),
            Ok(ServerStatus::Stopped) => self.start_server(),
            Ok(status) => Err(ServerError::StatusError(status, SlashCommand::Resume)),
            Err(e) => Err(e),
        }
    }

    /// Gets the status of the server
    fn get_status(&self) -> Result<ServerStatus, ServerError>;
}

#[async_trait]
impl ServerCommands for ServerType {
    async fn connect(&self) -> Result<String, ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Running) => match self {
                ServerType::Docker(docker) => docker.connect().await,
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Connect)),
            Err(e) => return Err(e),
        }
    }

    fn start_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Stopped) => match self {
                ServerType::Docker(docker) => docker.start_server(),
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Start)),
            Err(e) => return Err(e),
        }
    }

    fn stop_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Running) => match self {
                ServerType::Docker(docker) => docker.stop_server(),
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Stop)),
            Err(e) => return Err(e),
        }
    }

    fn restart_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Stopped) => self.start_server(),
            Ok(_) => match self {
                ServerType::Docker(docker) => docker.restart_server(),
            },
            Err(e) => return Err(e),
        }
    }

    fn pause_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Running) => match self {
                ServerType::Docker(docker) => docker.pause_server(),
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Pause)),
            Err(e) => return Err(e),
        }
    }

    fn unpause_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Paused) => match self {
                ServerType::Docker(docker) => docker.unpause_server(),
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Unpause)),
            Err(e) => return Err(e),
        }
    }

    fn resume_server(&self) -> Result<(), ServerError> {
        match self.get_status() {
            Ok(ServerStatus::Paused | ServerStatus::Stopped) => match self {
                ServerType::Docker(docker) => docker.resume_server(),
            },
            Ok(status) => return Err(ServerError::StatusError(status, SlashCommand::Resume)),
            Err(e) => return Err(e),
        }
    }

    fn get_status(&self) -> Result<ServerStatus, ServerError> {
        match self {
            ServerType::Docker(docker) => docker.get_status(),
        }
    }
}

#[async_trait]
impl ServerCommands for Docker {
    async fn connect(&self) -> Result<String, ServerError> {
        Ok(if self.connect.contains("$PUBLIC_IP") {
            let public_ip = reqwest::get("https://api.ipify.org")
                .await
                .map_err(|_| {
                    ServerError::CommandFailed(
                        SlashCommand::Connect,
                        "Failed to get public IP".to_string(),
                    )
                })?
                .text()
                .await
                .map_err(|_| {
                    ServerError::CommandFailed(
                        SlashCommand::Connect,
                        "Failed to get public IP".to_string(),
                    )
                })?;

            self.connect.replace("$PUBLIC_IP", &public_ip)
        } else {
            self.connect.clone()
        })
    }

    fn start_server(&self) -> Result<(), ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["start", &self.container_name])
            .output()
            .expect("Failed to start server");

        cmd.status
            .success()
            .then(|| ())
            .ok_or(ServerError::CommandFailed(
                SlashCommand::Start,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
    }

    fn stop_server(&self) -> Result<(), ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["stop", &self.container_name])
            .output()
            .expect("Failed to stop server");

        cmd.status
            .success()
            .then(|| ())
            .ok_or(ServerError::CommandFailed(
                SlashCommand::Stop,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
    }

    fn restart_server(&self) -> Result<(), ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["restart", &self.container_name])
            .output()
            .expect("Failed to restart server");

        cmd.status
            .success()
            .then(|| ())
            .ok_or(ServerError::CommandFailed(
                SlashCommand::Restart,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
    }

    fn pause_server(&self) -> Result<(), ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["pause", &self.container_name])
            .output()
            .expect("Failed to pause server");

        cmd.status
            .success()
            .then(|| ())
            .ok_or(ServerError::CommandFailed(
                SlashCommand::Pause,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
    }

    fn unpause_server(&self) -> Result<(), ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["unpause", &self.container_name])
            .output()
            .expect("Failed to pause server");

        cmd.status
            .success()
            .then(|| ())
            .ok_or(ServerError::CommandFailed(
                SlashCommand::Unpause,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
    }

    fn get_status(&self) -> Result<ServerStatus, ServerError> {
        let cmd = process::Command::new("docker")
            .args(&["inspect", &self.container_name])
            .output()
            .expect("Failed to get server status");

        if cmd.status.success() {
            let status = String::from_utf8(cmd.stdout).expect("Failed to read stdout");
            if status.contains("\"Status\": \"running\"") {
                Ok(ServerStatus::Running)
            } else if status.contains("\"Status\": \"paused\"") {
                Ok(ServerStatus::Paused)
            } else if status.contains("\"Status\": \"exited\"") {
                Ok(ServerStatus::Stopped)
            } else {
                Ok(ServerStatus::Unknown(status))
            }
        } else {
            Err(ServerError::CommandFailed(
                SlashCommand::Status,
                String::from_utf8(cmd.stderr).expect("Failed to read stderr"),
            ))
        }
    }
}
