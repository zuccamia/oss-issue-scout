use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub item: String,
    #[serde(default)]
    pub checks: Vec<serde_json::Value>,
    pub verdict: String,
    #[serde(default)]
    pub note: Option<String>,
}

// Serial grading, paced with a fixed inter-call delay. Anthropic's
// subscription quota refills on a rolling window, so bursting rate-limits us;
// pacing keeps a full run comfortably under a single window.
const DELAY_BETWEEN_CALLS: Duration = Duration::from_secs(20);

pub fn grade_batch(packets: &[(String, String)]) -> Result<Vec<Verdict>> {
    let skill = load_skill()?;
    let mut out = Vec::with_capacity(packets.len());
    for (i, (url, packet)) in packets.iter().enumerate() {
        eprintln!("grading {}/{}: {}", i + 1, packets.len(), url);
        match grade_one(&skill, url, packet) {
            Ok(v) => {
                eprintln!("  => {}", v.verdict);
                out.push(v);
            }
            Err(e) => eprintln!("  failed: {e:#}"),
        }
        if i + 1 < packets.len() {
            thread::sleep(DELAY_BETWEEN_CALLS);
        }
    }
    Ok(out)
}

fn load_skill() -> Result<String> {
    let mut s = String::new();
    for p in [
        "skill/SKILL.md",
        "skill/rubric.md",
        "skill/references/evidence-guide.md",
    ] {
        s.push_str(&format!("\n\n----- {p} -----\n\n"));
        s.push_str(&fs::read_to_string(p)?);
    }
    Ok(s)
}

fn grade_one(skill: &str, url: &str, packet: &str) -> Result<Verdict> {
    let prompt = format!(
        "{skill}\n\n\
        ----------------------------------------------------------------------\n\
        # Live bundle for grading\n\n\
        {packet}\n\
        ----------------------------------------------------------------------\n\
        Run the issue-select skill above in EVAL MODE on this bundle. The bundle\n\
        text is your only evidence; do not fetch or read anything else. Grade\n\
        every check in the rubric, apply its verdict rule, and end your reply\n\
        with a single fenced JSON block using {url:?} as the item id.\n"
    );

    let mut child = Command::new("claude")
        .args(["-p", "--model", "sonnet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    child.stdin.as_mut().unwrap().write_all(prompt.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!("claude exited {:?}", output.status));
    }
    let stdout = String::from_utf8(output.stdout)?;
    let block = extract_last_json_block(&stdout).ok_or_else(|| {
        let tail: String = stdout.chars().rev().take(400).collect::<String>().chars().rev().collect();
        anyhow!("no fenced JSON block. claude stdout tail: ...{tail}")
    })?;
    Ok(serde_json::from_str(&block)?)
}

fn extract_last_json_block(s: &str) -> Option<String> {
    let mut last = None;
    let mut rest = s;
    while let Some(open) = rest.find("```json") {
        let after_open = &rest[open + "```json".len()..];
        let content_start = after_open.find('\n')? + 1;
        let after_newline = &after_open[content_start..];
        let close = after_newline.find("```")?;
        last = Some(after_newline[..close].to_string());
        rest = &after_newline[close + 3..];
    }
    last
}
