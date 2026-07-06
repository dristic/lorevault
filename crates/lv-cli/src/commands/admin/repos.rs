use std::io::Write;

use lv_api_types::admin::AdminRepoSummary;
use tabwriter::TabWriter;

use crate::client::ApiClient;
use crate::pagination;

pub async fn list(
    client: &ApiClient,
    limit: Option<u32>,
    cursor: Option<String>,
) -> anyhow::Result<()> {
    let repos =
        pagination::collect::<AdminRepoSummary>(client, "/api/v1/admin/repos", limit, cursor)
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
