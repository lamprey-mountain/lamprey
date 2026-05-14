pub mod args;
pub mod config;

#[derive(Debug, strum::Display)]
pub enum AuthType {
    Token,
    PasswordUserId,
    PasswordEmail,
    Oauth,
}
