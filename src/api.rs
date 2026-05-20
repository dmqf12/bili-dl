use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::header::HeaderMap;
use serde_json::Value;
use crate::wbi::WbiKey;

pub fn client_builder() -> reqwest::ClientBuilder {
    let mut headers = HeaderMap::new();
    headers.insert("Referer", "https://www.bilibili.com/".parse().unwrap());
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .default_headers(headers)
}

pub async fn get_nav(client: &Client) -> Result<Value> {
    let resp: Value = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .send().await?
        .json().await?;
    Ok(resp)
}

pub async fn get_webpage(client: &Client, bvid: &str) -> Result<String> {
    let url = format!("https://www.bilibili.com/video/{}", bvid);
    let resp = client.get(&url).send().await?;
    resp.text().await.context("无法读取网页内容")
}

pub async fn get_playinfo(client: &Client, bvid: &str, cid: u64, wbi: &WbiKey) -> Result<Value> {
    let mut params = vec![
        ("bvid".to_string(), bvid.to_string()),
        ("cid".to_string(), cid.to_string()),
        ("fnval".to_string(), "4048".to_string()),
    ];
    let signed = wbi.sign(&mut params);

    let resp: Value = client
        .get("https://api.bilibili.com/x/player/wbi/playurl")
        .query(&signed)
        .send().await?
        .json().await?;

    Ok(resp["data"].clone())
}
