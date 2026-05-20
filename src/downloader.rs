use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use std::io::Write;

pub async fn download(client: &Client, url: &str, path: &str) -> Result<()> {
    if url.is_empty() {
        anyhow::bail!("下载地址为空");
    }

    let resp = client
        .get(url)
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await
        .context("网络请求失败")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("下载失败: HTTP {}", status);
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(path)
        .context(format!("无法创建文件: {}", path))?;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("下载过程中断")?;
        file.write_all(&chunk)
            .context(format!("写入文件失败: {}", path))?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            eprint!("\r  {}: {:.0}% ({}/{})",
                path,
                (downloaded as f64 / total as f64) * 100.0,
                format_size(downloaded),
                format_size(total));
        } else {
            eprint!("\r  {}: {}", path, format_size(downloaded));
        }
    }
    eprintln!();
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{}B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else { format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)) }
}
