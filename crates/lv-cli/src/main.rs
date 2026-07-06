mod client;
mod commands;
mod config;
mod error;
mod pagination;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

use client::ApiClient;
use config::CliConfig;

#[derive(Parser)]
#[command(
    name = "lorevault",
    version,
    about = "LoreVault CLI — manage users, repos, and your own account"
)]
struct Cli {
    /// Print debug logs (request/response details, etc.) to stdout.
    #[arg(short = 'd', long, global = true)]
    debug: bool,

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
    Repos {
        #[command(subcommand)]
        command: ReposCommand,
    },
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
enum ReposCommand {
    /// List repositories you own.
    List {
        #[command(flatten)]
        page: PageArgs,
    },
    /// Grant a user access to a repository, or change their existing role.
    AddUser {
        /// Repository in `owner/name` form.
        repo: String,
        username: String,
        #[arg(long, value_enum)]
        role: RepoRoleArg,
    },
    /// Revoke a user's access to a repository.
    RemoveUser {
        /// Repository in `owner/name` form.
        repo: String,
        username: String,
    },
}

/// Shared pagination flags for `list` subcommands. Omit both to auto-follow
/// every page and print one combined table; pass `--limit` to fetch a single
/// page and get a `--cursor` hint for the next one.
#[derive(clap::Args)]
struct PageArgs {
    /// Max rows to fetch per page. Omit to auto-follow every page and print
    /// one combined table.
    #[arg(long)]
    limit: Option<u32>,
    /// Resume from a cursor returned by a previous `--limit`'d call.
    #[arg(long)]
    cursor: Option<String>,
}

#[derive(Clone, ValueEnum)]
enum RepoRoleArg {
    Read,
    Write,
    Admin,
}

impl From<RepoRoleArg> for lv_api_types::repos::RepoRole {
    fn from(r: RepoRoleArg) -> Self {
        match r {
            RepoRoleArg::Read => Self::Read,
            RepoRoleArg::Write => Self::Write,
            RepoRoleArg::Admin => Self::Admin,
        }
    }
}

#[derive(Subcommand)]
enum TokensCommand {
    /// List your API tokens.
    List {
        #[command(flatten)]
        page: PageArgs,
    },
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
    List {
        #[command(flatten)]
        page: PageArgs,
    },
    /// Create a new user account.
    Create {
        username: String,
        email: String,
        /// Omit to be prompted (hidden input) instead of passing it on the
        /// command line, which would leak it into shell history.
        #[arg(long)]
        password: Option<String>,
    },
    /// Grant or revoke admin privileges for a user.
    SetAdmin {
        username: String,
        action: AdminAction,
    },
    /// Reset another user's password (forces them to change it at next login).
    ResetPassword {
        username: String,
        new_password: String,
    },
}

#[derive(Clone, ValueEnum)]
enum AdminAction {
    Grant,
    Revoke,
}

#[derive(Subcommand)]
enum AdminReposCommand {
    /// List every repository on the instance.
    List {
        #[command(flatten)]
        page: PageArgs,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.debug { "debug" } else { "off" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .without_time()
        .init();

    match cli.command {
        Command::Login {
            server,
            user,
            password,
        } => commands::login::run(server, user, password).await,
        Command::Logout => commands::logout::run(),
        Command::Health { server } => commands::health::run(server).await,
        Command::Whoami => commands::whoami::run(&build_client()?).await,
        Command::Passwd => commands::passwd::run(&build_client()?).await,
        Command::Repos { command } => {
            let client = build_client()?;
            match command {
                ReposCommand::List { page } => {
                    commands::repos::list(&client, page.limit, page.cursor).await
                }
                ReposCommand::AddUser {
                    repo,
                    username,
                    role,
                } => commands::repos::add_user(&client, repo, username, role.into()).await,
                ReposCommand::RemoveUser { repo, username } => {
                    commands::repos::remove_user(&client, repo, username).await
                }
            }
        }
        Command::Tokens { command } => {
            let client = build_client()?;
            match command {
                TokensCommand::List { page } => {
                    commands::tokens::list(&client, page.limit, page.cursor).await
                }
                TokensCommand::Create { name } => commands::tokens::create(&client, name).await,
            }
        }
        Command::Admin { command } => {
            let client = build_client()?;
            match command {
                AdminCommand::Users { command } => match command {
                    AdminUsersCommand::List { page } => {
                        commands::admin::users::list(&client, page.limit, page.cursor).await
                    }
                    AdminUsersCommand::Create {
                        username,
                        email,
                        password,
                    } => commands::admin::users::create(&client, username, email, password).await,
                    AdminUsersCommand::SetAdmin { username, action } => {
                        commands::admin::users::set_admin(
                            &client,
                            username,
                            matches!(action, AdminAction::Grant),
                        )
                        .await
                    }
                    AdminUsersCommand::ResetPassword {
                        username,
                        new_password,
                    } => {
                        commands::admin::users::reset_password(&client, username, new_password)
                            .await
                    }
                },
                AdminCommand::Repos { command } => match command {
                    AdminReposCommand::List { page } => {
                        commands::admin::repos::list(&client, page.limit, page.cursor).await
                    }
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
