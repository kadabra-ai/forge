use anyhow::Result;
use std::sync::LazyLock;

use regex::Regex;

static HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)^(#{1,4})\s*(?:<span id="([^"]+)"></span>)?\*\*([\d.]+)\s+(.+?)\*\*\s*$"#,
    )
    .expect("heading regex")
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClauseNode {
    pub id: String,
    pub title: String,
    pub anchor: String,
    pub level: u8,
    pub line: usize,
}

pub fn extract_clause_tree(src: &str) -> Result<Vec<ClauseNode>> {
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if let Some(caps) = HEADING.captures(line) {
            let level = caps.get(1).expect("level group").len() as u8;
            let anchor = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let id = caps
                .get(3)
                .expect("id group")
                .as_str()
                .trim_end_matches('.')
                .to_string();
            let title = caps.get(4).expect("title group").as_str().to_string();
            out.push(ClauseNode {
                id,
                title,
                anchor,
                level,
                line: i + 1,
            });
        }
    }
    Ok(out)
}