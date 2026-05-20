pub mod api;
pub mod cookie;
pub mod downloader;
pub mod extractor;
pub mod wbi;

pub use anyhow::{Context, Result};
pub use std::path::Path;

pub struct BiliDownloader {
    url_or_bvid: String,
    cookie_path: Option<String>,
    output_path: Option<String>,
}

impl BiliDownloader {
    pub fn new(url_or_bvid: &str) -> Self {
        Self {
            url_or_bvid: url_or_bvid.to_string(),
            cookie_path: None,
            output_path: None,
        }
    }

    pub fn cookie(mut self, path: &str) -> Self {
        self.cookie_path = Some(path.to_string());
        self
    }

    pub fn output(mut self, path: &str) -> Self {
        self.output_path = Some(path.to_string());
        self
    }

    pub async fn download(&self) -> Result<()> {
        // 1. 标准化 URL
        let mut url = self.url_or_bvid.clone();
        if !url.contains("bilibili.com") && !url.contains("b23.tv") {
            if url.contains("BV") {
                url = format!("https://www.bilibili.com/video/{}", url);
            } else {
                anyhow::bail!("无效BV号或URL");
            }
        }

        if url.contains("b23.tv") {
            let long_url = self.get_long_url(&url).await?;
            if long_url == "None" { anyhow::bail!("无法解析短链接"); }
            url = long_url;
        }

        let (bvid, page) = extractor::parse_url(&url)?;

        // 2. 构造 Client (Cookie 失败则降级)
        let mut builder = api::client_builder();
        if let Some(ref c_path) = self.cookie_path {
            match cookie::load_file(api::client_builder(), c_path) {
                Ok(b) => builder = b,
                Err(e) => eprintln!("警告: Cookie 加载失败 ({})，将尝试无 Cookie 下载...", e),
            }
        }
        let client = builder.build()?;
        // 3. 获取视频流信息
        let wbi_key = wbi::get_wbi_key(&client).await?;
        let video_info = extractor::extract_video_info(&client, &bvid, page).await?;
        println!("标题: {}\nBV号: {}\nCID: {}\n", video_info.title, video_info.bvid, video_info.cid);

        let play_info = api::get_playinfo(&client, &bvid, video_info.cid, &wbi_key).await?;
        let formats = extractor::extract_formats(&play_info);

        if formats.videos.is_empty() && formats.audios.is_empty() {
            anyhow::bail!("未找到可下载格式（可能需要登录，或检查 Cookie 是否有效）");
        }

        let best_video = formats.videos.iter()
            .max_by(|a, b| {
                a.bandwidth.unwrap_or(0).cmp(&b.bandwidth.unwrap_or(0))
                    .then(a.height.unwrap_or(0).cmp(&b.height.unwrap_or(0)))
                    .then(Self::score_codec(&b.codecs).cmp(&Self::score_codec(&a.codecs)))
            })
            .context("没有可用的视频格式")?;

        let best_audio = formats.audios.iter()
            .max_by_key(|f| f.bandwidth.unwrap_or(0))
            .context("没有可用的音频格式")?;

        // 4. 计算输出路径 (无效路径直接报错)
        let safe_title = video_info.title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let final_mp4 = match &self.output_path {
            Some(o) => {
                let path = Path::new(o);
                if o.ends_with('/') || o.ends_with('\\') || path.is_dir() {
                    if !path.exists() { anyhow::bail!("指定的输出目录不存在: {}", o); }
                    format!("{}/{}.mp4", o.trim_end_matches(['/', '\\']), safe_title)
                } else {
                    if let Some(parent) = path.parent() {
                        if !parent.as_os_str().is_empty() && !parent.exists() {
                            anyhow::bail!("指定的输出路径父目录不存在: {:?}", parent);
                        }
                    }
                    o.clone()
                }
            }
            None => format!("{}.mp4", safe_title),
        };

        // 5. 生成临时文件路径
        let stem = final_mp4.strip_suffix(".mp4").unwrap_or(&final_mp4);
        let video_temp = format!("{}_video.m4s", stem);
        let audio_temp = format!("{}_audio.m4s", stem);

        println!("已选: {}x{} @ {}kbps + {}kbps 音频 → {}",
            best_video.width.unwrap_or(0), best_video.height.unwrap_or(0),
            best_video.bandwidth.unwrap_or(0) / 1000, best_audio.bandwidth.unwrap_or(0) / 1000,
            final_mp4
        );

        // 6. 下载与合并
        println!("下载中...");
        downloader::download(&client, &best_video.url, &video_temp).await?;
        downloader::download(&client, &best_audio.url, &audio_temp).await?;

        println!("合并中...");
        let status = std::process::Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y", "-i", &video_temp, "-i", &audio_temp, "-c", "copy", &final_mp4])
            .status()
            .context("未找到 ffmpeg，请先安装 ffmpeg")?;

        if status.success() {
            let _ = std::fs::remove_file(&video_temp);
            let _ = std::fs::remove_file(&audio_temp);
            println!("完成: {}", final_mp4);
            Ok(())
        } else {
            anyhow::bail!("ffmpeg 合并失败，临时文件保留:\n  {}\n  {}", video_temp, audio_temp);
        }
    }

    async fn get_long_url(&self, short_url: &str) -> Result<String> {
        let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()?;
        let response = client.head(short_url).send().await?;
        if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
            Ok(location.to_str()?.to_string())
        } else {
            Ok("None".to_string())
        }
    }

    fn score_codec(codec: &str) -> u32 {
        let c = codec.to_lowercase();
        if c.starts_with("avc") { 3 } else if c.starts_with("av01") { 2 } else if c.starts_with("hev") { 1 } else { 0 }
    }
}
