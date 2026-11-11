use std::fmt;

use anyhow::Result;
use clap::Parser;
use lamprey_cli::args::{Args, AuthCommand, Command};
use lamprey_cli::auth;
use lamprey_cli::config::LyConfig;
use serde::Serialize;

/// output of a command
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Output {
    Login(auth::LoginResult),
    Logout(auth::LogoutResult),
    Check(Box<auth::CheckResult>),
    List(auth::ListResult),
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Output::Login(result) => fmt::Display::fmt(result, f),
            Output::Logout(result) => fmt::Display::fmt(result, f),
            Output::Check(result) => fmt::Display::fmt(result, f),
            Output::List(result) => fmt::Display::fmt(result, f),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let Args {
        config,
        login,
        json,
        command,
    } = Args::parse();
    let login = login.as_deref();
    let mut config = LyConfig::open(config)?;

    let output = match command {
        Command::Auth { command } => match command {
            AuthCommand::Login {
                api_url,
                token,
                name,
                register,
                scope,
                registration_token,
            } => Output::Login(
                auth::login(
                    &mut config,
                    api_url,
                    token,
                    name,
                    register,
                    scope,
                    registration_token,
                )
                .await?,
            ),
            AuthCommand::Logout => Output::Logout(auth::logout(&mut config, login).await?),
            AuthCommand::Check => Output::Check(Box::new(auth::check(&mut config, login).await?)),
            AuthCommand::List => Output::List(auth::list(&config)),
        },
        Command::Message { .. } => todo!(),
        Command::Channel { .. } => todo!(),
        Command::Media { .. } => todo!(),
        Command::Events {} => todo!(),
    };

    emit(json, &output)
}

fn emit<T: Serialize + fmt::Display>(json: bool, value: &T) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{value}");
    }

    Ok(())
}
