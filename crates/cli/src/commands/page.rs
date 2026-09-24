use std::ops::Range;

use crate::error::CommandError;

pub fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| error.to_string())
        .and_then(|limit| {
            (limit > 0)
                .then_some(limit)
                .ok_or_else(|| "limit must be positive".into())
        })
}

pub struct Page {
    pub range: Range<usize>,
    pub truncated: bool,
    pub total: u64,
    pub next_cursor: Option<u64>,
}

pub fn bounds(total: usize, limit: usize, cursor: usize) -> crate::error::Result<Page> {
    if cursor > total {
        return Err(CommandError::evaluation(
            "invalid-cursor",
            format!("cursor {cursor} exceeds total {total}"),
            Some("omit --cursor to start from the beginning".into()),
        ));
    }
    let end = cursor.saturating_add(limit).min(total);
    let truncated = end < total;
    Ok(Page {
        range: cursor..end,
        truncated,
        total: total as u64,
        next_cursor: truncated.then_some(end as u64),
    })
}
