use regex::Regex;

pub struct Blacklist {
    patterns: Vec<Regex>,
}
use anyhow::{Context, Result};
use tokio::fs;

impl Blacklist {
    pub async fn from_file(path: Option<&String>) -> Result<Self> {
        if path.is_none() {
            return Ok(Self { patterns: vec![] });
        }
        let path = path.unwrap();
        let content = fs::read_to_string(path)
            .await
            .context("failed to read blacklist file")?;

        let mut patterns = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let regex = Regex::new(line)
                .with_context(|| format!("invalid regex at line {}: {}", line_no + 1, line))?;

            patterns.push(regex);
        }

        Ok(Self { patterns })
    }

    pub fn is_blocked(&self, url: &str) -> bool {
        self.patterns.iter().any(|r| r.is_match(url))
    }
}
