use anyhow::{Context, Result};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;

pub struct VideoInfo {
    pub bvid: String,
    pub title: String,
    pub cid: u64,
}

pub struct Format {
    pub url: String,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub bandwidth: Option<u64>,
    pub filesize: Option<u64>,
    pub codecs: String,
}

pub struct Formats {
    pub videos: Vec<Format>,
    pub audios: Vec<Format>,
}

pub fn parse_url(url: &str) -> Result<(String, Option<u32>)> {
    let re = Regex::new(r"bilibili\.com/video/(?P<bvid>[aAbB][vV][A-Za-z0-9]+)").unwrap();
    let caps = re.captures(url).context("无法从 URL 提取 BV 号，请确认是 https://www.bilibili.com/video/BVxxx 格式")?;
    let bvid = caps["bvid"].to_string();
    let page = url::Url::parse(url)
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "p").and_then(|(_, v)| v.parse::<u32>().ok()));
    Ok((bvid, page))
}

fn extract_json_obj(text: &str, start_marker: &str) -> Option<String> {
    let pos = text.find(start_marker)?;
    let after = &text[pos + start_marker.len()..];
    let mut depth = 0;
    let mut started = false;
    let mut end = 0;
    for (i, ch) in after.char_indices() {
        if ch == '{' { depth += 1; started = true; }
        else if ch == '}' {
            depth -= 1;
            if started && depth == 0 { end = i + ch.len_utf8(); break; }
        }
    }
    if end == 0 { None } else { Some(after[..end].to_string()) }
}

fn extract_initial_state(html: &str) -> Result<Value> {
    let json_str = extract_json_obj(html, "window.__INITIAL_STATE__=")
        .or_else(|| extract_json_obj(html, "__INITIAL_STATE__="))
        .context("无法解析网页内容，可能触发了反爬或被重定向。试试带 cookie 登录")?;
    serde_json::from_str(&json_str.replace("undefined", "null"))
        .context("无法解析页面 JSON 数据")
}

pub async fn extract_video_info(client: &Client, bvid: &str, page: Option<u32>) -> Result<VideoInfo> {
    let html = crate::api::get_webpage(client, bvid).await?;
    let state = extract_initial_state(&html)?;

    let video_data = &state["videoData"];

    if video_data.is_null() {
        // 可能是被重定向到番剧页面
        if let Some(redirect) = state["error"].as_object() {
            anyhow::bail!("服务器返回错误: {:?}", redirect);
        }
        anyhow::bail!("视频信息为空，可能视频不存在或被限区");
    }

    let title = video_data["title"].as_str().unwrap_or("未知标题").to_string();
    let cid = if let Some(p) = page {
        video_data["pages"].as_array()
            .and_then(|pages| pages.get((p - 1) as usize))
            .and_then(|p| p["cid"].as_u64())
            .unwrap_or_else(|| video_data["cid"].as_u64().unwrap_or(0))
    } else {
        video_data["cid"].as_u64().unwrap_or(0)
    };

    if cid == 0 {
        anyhow::bail!("无法获取视频 CID");
    }

    Ok(VideoInfo { bvid: bvid.to_string(), title, cid })
}

pub fn extract_formats(play_info: &Value) -> Formats {
    let mut videos = Vec::new();
    let mut audios = Vec::new();

    if let Some(arr) = play_info["dash"]["video"].as_array() {
        for v in arr {
            videos.push(Format {
                url: v["baseUrl"].as_str().or(v["base_url"].as_str()).unwrap_or("").to_string(),
                width: v["width"].as_u64(),
                height: v["height"].as_u64(),
                bandwidth: v["bandwidth"].as_u64(),
                filesize: v["size"].as_u64(),
                codecs: v["codecs"].as_str().unwrap_or("unknown").to_string(),
            });
        }
    }

    let pa = |list: &Vec<Value>, audios: &mut Vec<Format>| {
        for a in list {
            audios.push(Format {
                url: a["baseUrl"].as_str().or(a["base_url"].as_str()).unwrap_or("").to_string(),
                width: None, height: None,
                bandwidth: a["bandwidth"].as_u64(),
                filesize: a["size"].as_u64(),
                codecs: a["codecs"].as_str().unwrap_or("unknown").to_string(),
            });
        }
    };

    if let Some(arr) = play_info["dash"]["audio"].as_array() { pa(arr, &mut audios); }
    if let Some(arr) = play_info["dash"]["dolby"]["audio"].as_array() { pa(arr, &mut audios); }
    if let Some(flac) = play_info["dash"]["flac"].as_object() {
        if let Some(audio) = flac.get("audio") {
            audios.push(Format {
                url: audio["baseUrl"].as_str().or(audio["base_url"].as_str()).unwrap_or("").to_string(),
                width: None, height: None,
                bandwidth: audio["bandwidth"].as_u64(),
                filesize: audio["size"].as_u64(),
                codecs: audio["codecs"].as_str().unwrap_or("flac").to_string(),
            });
        }
    }

    Formats { videos, audios }
}
