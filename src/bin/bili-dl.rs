use anyhow::{Context, Result};
use bili_dl::{api, wbi, extractor, downloader, cookie};

async fn get_long_url(short_url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client.head(short_url).send().await?;
    // 从响应头的 Location 中获取长链接
    if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
        let long_url = location.to_str()?.to_string();
        Ok(long_url)
    } else {
        Ok("None".to_string())
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let mut url = args[1].clone();
    if url == "-h" || url == "--help" {
        print_usage();
        return Ok(());
    }
    if !url.contains("bilibili.com") && !url.contains("b23.tv") {
        if url.contains("BV") {
            url = format!("https://www.bilibili.com/video/{}", url);
        } else {
            anyhow::bail!("无效BV号");
        }
    }

    if url.contains("b23.tv") {
        if let Ok(long_url) = get_long_url(&url).await {
            if long_url == "None" {
                anyhow::bail!("无效链接");
            }
            url = long_url;
        }
    }

    let (bvid, page) = extractor::parse_url(&url)?;

    let mut builder = api::client_builder();
    let mut output_name = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output_name = Some(
                    args.get(i).context("-o/--output 需要指定文件名或目录")?.clone(),
                );
            }
            "--cookie" => {
                i += 1;
                let path = args.get(i).context("--cookie 需要指定文件路径")?;
                builder = cookie::load_file(builder, path)?;
            }
            _ => {
                anyhow::bail!("未知参数: {}", args[i]);
            }
        }
        i += 1;
    }

    let client = builder.build()?;
    let wbi_key = wbi::get_wbi_key(&client).await?;
    let video_info = extractor::extract_video_info(&client, &bvid, page).await?;
    println!("标题: {}", video_info.title);
    println!("BV号: {}", video_info.bvid);
    println!("CID: {}", video_info.cid);
    println!();

    let play_info = api::get_playinfo(&client, &bvid, video_info.cid, &wbi_key).await?;
    let formats = extractor::extract_formats(&play_info);

    if formats.videos.is_empty() && formats.audios.is_empty() {
        anyhow::bail!("未找到任何可下载的格式（可能需要登录，试试 --cookie）");
    }

    println!("可用格式:");
    println!("--- 视频 ---");
    for f in &formats.videos {
        println!(
            "  {:>4}x{:<4} {:>5}kbps  {}",
            f.width.unwrap_or(0),
            f.height.unwrap_or(0),
            f.bandwidth.unwrap_or(0) / 1000,
            f.codecs,
        );
    }
    println!("--- 音频 ---");
    for f in &formats.audios {
        println!(
            "  {:>6}kbps  {:>8}  {}",
            f.bandwidth.unwrap_or(0) / 1000,
            format_size(f.filesize.unwrap_or(0)),
            f.codecs,
        );
    }

    let best_video = formats.videos.iter()
        .max_by(|a, b| {
            a.bandwidth.unwrap_or(0).cmp(&b.bandwidth.unwrap_or(0))
                .then(a.height.unwrap_or(0).cmp(&b.height.unwrap_or(0)))
                .then(score_codec(&b.codecs).cmp(&score_codec(&a.codecs)))
        })
        .context("没有可用的视频格式")?;

    let best_audio = formats.audios.iter()
        .max_by_key(|f| f.bandwidth.unwrap_or(0))
        .context("没有可用的音频格式")?;

    let safe_title = video_info.title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let output_path = output_name.map(|o| {
        if o.ends_with('/') || o.ends_with('\\') || std::path::Path::new(&o).is_dir() {
            format!("{}/{}.mp4", o.trim_end_matches(['/', '\\']), safe_title)
        } else {
            o
        }
    }).unwrap_or_else(|| format!("{}.mp4", safe_title));

    let stem = output_path.strip_suffix(".mp4").unwrap_or(&output_path);
    let video_path = format!("{}_video.m4s", stem);
    let audio_path = format!("{}_audio.m4s", stem);

    println!();
    println!("已选: {}x{} @ {}kbps + {}kbps 音频 → {}",
        best_video.width.unwrap_or(0), best_video.height.unwrap_or(0),
        best_video.bandwidth.unwrap_or(0) / 1000, best_audio.bandwidth.unwrap_or(0) / 1000,
        output_path,
    );

    println!("下载中...");
    downloader::download(&client, &best_video.url, &video_path).await?;
    println!("视频下载完成");
    downloader::download(&client, &best_audio.url, &audio_path).await?;
    println!("音频下载完成");

    println!("合并中...");
    let status = std::process::Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-i", &video_path, "-i", &audio_path, "-c", "copy", &output_path])
        .status()
        .context("未找到 ffmpeg，请安装: apt install ffmpeg")?;

    if status.success() {
        let _ = std::fs::remove_file(&video_path);
        let _ = std::fs::remove_file(&audio_path);
        println!("完成: {}", output_path);
    } else {
        anyhow::bail!("ffmpeg 合并失败，临时文件保留:\n  {}\n  {}", video_path, audio_path);
    }

    Ok(())
}

fn print_usage() {
    eprintln!("bili-dl —— B站视频下载器 (Rust)");
    eprintln!();
    eprintln!("用法:");
    eprintln!("  bili-dl <URL> [-o <路径>] [--cookie <文件>]");
    eprintln!("  bili-dl <BV号> [-o <路径>] [--cookie <文件>]");
    eprintln!();
    eprintln!("选项:");
    eprintln!("  -o, --output <路径>  输出文件或目录（目录需以 / 结尾，文件名用视频标题）");
    eprintln!("  --cookie <文件>      登录 cookie（JSON 或 Netscape 格式）");
    eprintln!("  -h, --help           显示此帮助");
    eprintln!();
    eprintln!("示例:");
    eprintln!("  bili-dl BVxxx");
    eprintln!("  bili-dl https://www.bilibili.com/video/BVxxx");
    eprintln!("  bili-dl https://www.bilibili.com/video/BVxxx -o 视频.mp4");
    eprintln!("  bili-dl https://www.bilibili.com/video/BVxxx -o ./downloads/ --cookie cookies.json");
}

fn score_codec(codec: &str) -> u32 {
    let c = codec.to_lowercase();
    if c.starts_with("avc") { 3 }
    else if c.starts_with("av01") { 2 }
    else if c.starts_with("hev") { 1 }
    else { 0 }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{}B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else { format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)) }
}
