use std::io::Write;

use lv_api_types::repos::{RepoRole, SetRepoUserRequest};
use lv_api_types::users::RepoSummary;
use tabwriter::TabWriter;

use crate::client::ApiClient;

pub async fn list(client: &ApiClient) -> anyhow::Result<()> {
    let repos = client
        .get::<Vec<RepoSummary>>("/api/v1/users/me/repos")
        .await?;

    let mut tw = TabWriter::new(std::io::stdout());
    writeln!(tw, "ID\tNAME\tDESCRIPTION\tVISIBILITY\tDEFAULT")?;

    for repo in repos {
        let description = repo.description.unwrap_or_default().replace(['\t', '\n', '\r'], " ");
        writeln!(
            tw,
            "{}\t{}\t{:.20}\t{}\t{}",
            repo.id, repo.name, description, repo.visibility, repo.default_branch
        )?;
    }

    tw.flush()?;

    Ok(())
}

/// Grants `username` `role` on `repo` (or changes their existing role).
/// `repo` must be in `owner/name` form, matching how the server identifies
/// repos everywhere else — no UUIDs involved.
pub async fn add_user(
    client: &ApiClient,
    repo: String,
    username: String,
    role: RepoRole,
) -> anyhow::Result<()> {
    let (owner, name) = split_repo(&repo)?;
    client
        .post_no_content(
            &format!("/api/v1/repos/{owner}/{name}/users/{username}"),
            &SetRepoUserRequest { role },
        )
        .await?;

    println!("Granted {username} {role} access to {repo}.");

    Ok(())
}

/// Revokes `username`'s access to `repo` (`owner/name` form).
pub async fn remove_user(client: &ApiClient, repo: String, username: String) -> anyhow::Result<()> {
    let (owner, name) = split_repo(&repo)?;
    client
        .delete_no_content(&format!("/api/v1/repos/{owner}/{name}/users/{username}"))
        .await?;

    println!("Removed {username}'s access to {repo}.");

    Ok(())
}

fn split_repo(repo: &str) -> anyhow::Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(owner, name)| !owner.is_empty() && !name.is_empty())
        .ok_or_else(|| anyhow::anyhow!("expected repo in `owner/name` form, got `{repo}`"))
}
