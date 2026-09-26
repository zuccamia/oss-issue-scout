use chrono::NaiveDate;

use crate::payload;

// True if today's top accept URL was not among yesterday's accepts.
pub fn is_new_top(today: NaiveDate, top_url: &str) -> bool {
    let Some(y) = today.pred_opt() else { return true };
    match payload::load(y) {
        Ok(Some(p)) => !p.accepts.iter().any(|a| a.url == top_url),
        _ => true,
    }
}
