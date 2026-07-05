mod client;
mod commands;
mod config;
mod error;

use clap::{Parser, Subcommand, ValueEnum};

use client::ApiClient;
use config::CliConfig;

#[derive(Parser)]
#[command(name = "lorevault", version, about = "LoreVault CLI — manage users, repos, and your own account")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Authenticate against a LoreVault server and save the session token.
    Login {
        /// Base URL of the LoreVault server, e.g. https://lorevault.example.com
        server: String,
        #[arg(long)]
        user: String,
        /// Omit to be prompted (hidden input) instead of passing it on the
        /// command line, which would leak it into shell history.
        #[arg(long)]
        password: Option<String>,
    },
    /// Clear the saved session token.
    Logout,
    /// Check a server's health. Doesn't require being logged in.
    Health {
        /// Server to check; defaults to the currently logged-in server.
        server: Option<String>,
    },
    /// Show the currently authenticated user.
    Whoami,
    /// Change your own password.
    Passwd,
    /// Repositories you own.
    Repos,
    /// Manage your own API tokens.
    Tokens {
        #[command(subcommand)]
        command: TokensCommand,
    },
    /// Admin-only: manage users and repos instance-wide.
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
}

#[derive(Subcommand)]
enum TokensCommand {
    /// List your API tokens.
    List,
    /// Create a new API token (the raw value is shown once).
    Create { name: String },
}

#[derive(Subcommand)]
enum AdminCommand {
    /// Manage users on this instance.
    Users {
        #[command(subcommand)]
        command: AdminUsersCommand,
    },
    /// View repositories on this instance.
    Repos {
        #[command(subcommand)]
        command: AdminReposCommand,
    },
}

#[derive(Subcommand)]
enum AdminUsersCommand {
    /// List every user on the instance.
    List,
    /// Grant or revoke admin privileges for a user.
    SetAdmin { username: String, action: AdminAction },
    /// Reset another user's password (forces them to change it at next login).
    ResetPassword { username: String, new_password: String },
}

#[derive(Clone, ValueEnum)]
enum AdminAction {
    Grant,
    Revoke,
}

#[derive(Subcommand)]
enum AdminReposCommand {
    /// List every repository on the instance.
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Login { server, user, password } => commands::login::run(server, user, password).await,
        Command::Logout => commands::logout::run(),
        Command::Health { server } => commands::health::run(server).await,
        Command::Whoami => commands::whoami::run(&build_client()?).await,
        Command::Passwd => commands::passwd::run(&build_client()?).await,
        Command::Repos => commands::repos::list(&build_client()?).await,
        Command::Tokens { command } => {
            let client = build_client()?;
            match command {
                TokensCommand::List => commands::tokens::list(&client).await,
                TokensCommand::Create { name } => commands::tokens::create(&client, name).await,
            }
        }
        Command::Admin { command } => {
            let client = build_client()?;
            match command {
                AdminCommand::Users { command } => match command {
                    AdminUsersCommand::List => commands::admin::users::list(&client).await,
                    AdminUsersCommand::SetAdmin { username, action } => {
                        commands::admin::users::set_admin(&client, username, matches!(action, AdminAction::Grant))
                            .await
                    }
                    AdminUsersCommand::ResetPassword { username, new_password } => {
                        commands::admin::users::reset_password(&client, username, new_password).await
                    }
                },
                AdminCommand::Repos { command } => match command {
                    AdminReposCommand::List => commands::admin::repos::list(&client).await,
                },
            }
        }
    }
}

/// Builds an API client from the saved config. Every command except `login`
/// and `logout` needs one.
fn build_client() -> anyhow::Result<ApiClient> {
    let cfg = CliConfig::load()?;
    let server = cfg
        .server
        .clone()
        .ok_or_else(|| anyhow::anyhow!("not logged in — run `lorevault login` first"))?;
    Ok(ApiClient::new(server, cfg.token))
}
