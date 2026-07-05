use std::io::Write;

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
