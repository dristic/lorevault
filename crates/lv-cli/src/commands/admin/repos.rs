use std::io::Write;

use lv_api_types::admin::AdminRepoSummary;
use tabwriter::TabWriter;

use crate::client::ApiClient;

pub async fn list(client: &ApiClient) -> anyhow::Result<()> {
    let repos = client.get::<Vec<AdminRepoSummary>>("/api/v1/admin/repos")
        .await?;
    
    let mut tw = TabWriter::new(std::io::stdout());
    writeln!(tw, "ID\tNAME\tOWNER\tDESCRIPTION\tVISIBILITY\tDEFAULT")?;

    for repo in repos {
        let description = repo.description.unwrap_or_default().replace(['\t', '\n', '\r'], " ");
        writeln!(
            tw,
            "{}\t{}\t{}\t{:.20}\t{}\t{}",
            repo.id, repo.name, repo.owner, description, repo.visibility, repo.default_branch
        )?;
    }

    tw.flush()?;

    Ok(())
}
