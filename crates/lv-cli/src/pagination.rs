use serde::de::DeserializeOwned;

use crate::client::ApiClient;

/// Fetches a paginated list endpoint. If the caller passed an explicit
/// `--limit`/`--cursor`, only one page is fetched and any remaining cursor is
/// printed as a hint. Otherwise every page is followed automatically so the
/// command's default behavior is "show me everything".
pub async fn collect<T: DeserializeOwned>(
    client: &ApiClient,
    path: &str,
    limit: Option<u32>,
    cursor: Option<String>,
) -> anyhow::Result<Vec<T>> {
    let manual = limit.is_some() || cursor.is_some();
    let mut items = Vec::new();
    let mut next_cursor = cursor;

    loop {
        let page = client
            .get_page::<T>(path, limit, next_cursor.as_deref())
            .await?;
        items.extend(page.items);
        next_cursor = page.next_cursor;

        if manual || next_cursor.is_none() {
            break;
        }
    }

    if manual {
        if let Some(cursor) = &next_cursor {
            eprintln!("more results available — pass `--cursor {cursor}` to continue");
        }
    }

    Ok(items)
}
