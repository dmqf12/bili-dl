use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct JsonCookie {
    domain: String,
    name: String,
    value: String,
    path: Option<String>,
}

pub fn load_file(builder: reqwest::ClientBuilder, path: &str) -> Result<reqwest::ClientBuilder> {
    let content = fs::read_to_string(path)
        .context(format!("无法读取 cookie 文件: {}", path))?;

    let cookies = if content.trim().starts_with('[') || content.trim().starts_with('{') {
        parse_json(&content)?
    } else {
        parse_netscape(&content)
    };

    let provider = CookieProvider { cookies };
    Ok(builder.cookie_provider(std::sync::Arc::new(provider)))
}

fn parse_netscape(content: &str) -> Vec<CookieEntry> {
    content.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let parts: Vec<&str> = l.split('\t').collect();
            if parts.len() >= 7 {
                Some(CookieEntry {
                    domain: parts[0].to_string(),
                    name: parts[5].to_string(),
                    value: parts[6].to_string(),
                    path: Some(parts[2].to_string()),
                })
            } else { None }
        })
        .collect()
}

fn parse_json(content: &str) -> Result<Vec<CookieEntry>> {
    let entries: Vec<JsonCookie> = serde_json::from_str(content)
        .context("无法解析 JSON cookie")?;
    Ok(entries.into_iter().map(|c| CookieEntry {
        domain: c.domain,
        name: c.name,
        value: c.value,
        path: c.path,
    }).collect())
}

#[derive(Debug, Clone)]
struct CookieEntry {
    domain: String,
    name: String,
    value: String,
    path: Option<String>,
}

#[derive(Debug)]
struct CookieProvider {
    cookies: Vec<CookieEntry>,
}

impl reqwest::cookie::CookieStore for CookieProvider {
    fn set_cookies(
        &self,
        _headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>,
        _url: &url::Url,
    ) {}

    fn cookies(&self, url: &url::Url) -> Option<reqwest::header::HeaderValue> {
        let host = url.host_str()?;
        let url_path = url.path();
        let mut result = String::new();

        for c in &self.cookies {
            let host_match = c.domain == host
                || (c.domain.starts_with('.') && host.ends_with(&c.domain[1..]))
                || (c.domain.starts_with('.') && host == &c.domain[1..]);

            let path_match = c.path.as_deref().map_or(true, |p| {
                p == "/" || url_path == p || url_path.starts_with(p)
            });

            if host_match && path_match {
                if !result.is_empty() { result.push_str("; "); }
                result.push_str(&format!("{}={}", c.name, c.value));
            }
        }

        if result.is_empty() { None }
        else { reqwest::header::HeaderValue::from_str(&result).ok() }
    }
}
