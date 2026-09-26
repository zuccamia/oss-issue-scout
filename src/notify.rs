use anyhow::Result;
use serde_json::json;

pub fn slack(new_top: &str) -> Result<()> {
    let Ok(webhook) = std::env::var("SLACK_WEBHOOK_URL") else {
        return Ok(());
    };
    let body = json!({
        "text": format!("scout: new top candidate\n<{new_top}|{new_top}>"),
    });
    let _ = ureq::post(&webhook).send_json(body);
    Ok(())
}
