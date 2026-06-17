use std::path::PathBuf;

use clap::Parser;
use demand::{DemandOption, Input, Select};
use figment::providers::{Env, Format, Toml};
use lamprey_cli::AuthType;
use lamprey_cli::args::ly::{Args, AuthCommand, Command};
use lamprey_cli::config::LyConfig;

type Result<T> = std::result::Result<T, anyhow::Error>;

#[tokio::main]
async fn main() {
    if let Err(err) = main_inner().await {
        eprintln!("error: {err}");
    }
}

async fn main_inner() -> Result<()> {
    let args = Args::parse();

    let config_path = if let Some(config) = args.config {
        config
    } else {
        dirs::config_dir()
            .map(|d| d.join("ly").join("config.toml"))
            .expect("config dir not found")
    };

    let config: LyConfig = figment::Figment::new()
        .merge(Toml::file(config_path.clone()))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    match args.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => {
                let api_url = Input::new("api url")
                    .placeholder("http://localhost:4000")
                    .run()?;

                let auth_type = Select::new("auth type")
                    .description("how do you want to authenticate to the server")
                    .option(DemandOption::new(AuthType::Token).label("token"))
                    .option(DemandOption::new(AuthType::PasswordUserId).label("password + user id"))
                    .option(DemandOption::new(AuthType::PasswordEmail).label("password + email"))
                    .option(DemandOption::new(AuthType::Oauth).label("oauth"))
                    .run()?;

                let (token, name) = match auth_type {
                    AuthType::Token => {
                        let token = Input::new("token")
                            .placeholder("secret-uuid-here")
                            .password(true)
                            .run()?;
                        let name = Input::new("session name")
                            .placeholder("my cool session")
                            .run()?;
                        (token, name)
                    }
                    AuthType::PasswordUserId => {
                        todo!()
                    }
                    AuthType::PasswordEmail => {
                        todo!()
                    }
                    AuthType::Oauth => {
                        todo!()
                    }
                };

                let login = lamprey_cli::config::Login {
                    api_url,
                    token,
                    name,
                    default: false,
                };

                let mut logins = config.logins;
                logins.push(login);

                save_config(&config_path, &LyConfig { logins })?;
            }
            AuthCommand::Logout => todo!(),
        },
        Command::Message { command } => todo!(),
        Command::Channel { command } => todo!(),
        Command::Media { command } => todo!(),
        Command::Redex { command } => todo!(),
        Command::Events {} => todo!(),
    };

    Ok(())
}

fn save_config(config_path: &PathBuf, config: &LyConfig) -> Result<()> {
    let toml = toml::to_string_pretty(config)?;
    std::fs::write(config_path, toml)?;
    Ok(())
}
