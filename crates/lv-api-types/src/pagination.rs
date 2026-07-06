use serde::{Deserialize, Serialize};

/// Query parameters accepted by every paginated `GET` list endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct PageParams {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

/// Wire envelope returned by every paginated `GET` list endpoint. `next_cursor`
/// is present iff there are more rows to fetch — pass it back as `cursor` to
/// get the next page.
#[derive(Debug, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
