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
    video_only: bool,
    audio_only: bool,
    no_merge: bool,
}

impl BiliDownloader {
    pub fn new(url_or_bvid: &str) -> Self {
        Self {
            url_or_bvid: url_or_bvid.to_string(),
            cookie_path: None,
            output_path: None,
            video_only: false,
            audio_only: false,
            no_merge: false,
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

    pub fn video_only(mut self, v: bool) -> Self {
        self.video_only = v;
        self
    }

    pub fn audio_only(mut self, v: bool) -> Self {
        self.audio_only = v;
        self
    }

    pub fn no_merge(mut self, v: bool) -> Self {
        self.no_merge = v;
        self
    }

    pub async fn download(&self) -> Result<()> {
        if self.video_only && self.audio_only {
            anyhow::bail!("--video-only 和 --audio-only 不能同时使用");
        }

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

        // 2. 构造 Client
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

        // 4. 计算输出路径
        let safe_title = video_info.title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let final_mp4 = match &self.output_path {
            Some(o) => {
                let path = Path::new(o);
                if o.ends_with('/') || o.ends_with('\\') || path.is_dir() {
                    if !path.exists() { anyhow::bail!("指定的输出目录不存在: {}", o); }
                    format!("{}/{}.mp4", o.trim_end_matches(['/', '\\']), safe_title)
                } else {
                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty() && !parent.exists() {
                            anyhow::bail!("指定的输出路径父目录不存在: {:?}", parent);
                        }
                    o.clone()
                }
            }
            None => format!("{}.mp4", safe_title),
        };
        let stem = final_mp4.strip_suffix(".mp4").unwrap_or(&final_mp4);

        // 5. 下载视频
        if !self.audio_only {
            let best_video = formats.videos.iter()
                .max_by(|a, b| {
                    a.bandwidth.unwrap_or(0).cmp(&b.bandwidth.unwrap_or(0))
                        .then(a.height.unwrap_or(0).cmp(&b.height.unwrap_or(0)))
                        .then(Self::score_codec(&b.codecs).cmp(&Self::score_codec(&a.codecs)))
                })
                .context("没有可用的视频格式")?;

            let video_path = format!("{}_video.mp4", stem);
            println!("已选视频: {}x{} @ {}kbps",
                best_video.width.unwrap_or(0), best_video.height.unwrap_or(0),
                best_video.bandwidth.unwrap_or(0) / 1000);
            println!("下载视频中...");
            downloader::download(&client, &best_video.url, &video_path).await?;

            if self.video_only {
                // 只下载视频：重命名为最终输出
                std::fs::rename(&video_path, &final_mp4)
                    .context("重命名视频文件失败")?;
                println!("完成: {}", final_mp4);
                return Ok(());
            }

            // 下载音频
            let best_audio = formats.audios.iter()
                .max_by_key(|f| f.bandwidth.unwrap_or(0))
                .context("没有可用的音频格式")?;
            let audio_path = format!("{}_audio.mp4", stem);
            println!("已选音频: {}kbps", best_audio.bandwidth.unwrap_or(0) / 1000);
            println!("下载音频中...");
            downloader::download(&client, &best_audio.url, &audio_path).await?;

            if self.no_merge {
                println!("保留临时文件（--no-merge）:\n  {}\n  {}", video_path, audio_path);
                return Ok(());
            }

            // 合并
            println!("纯Rust合成中...");
            Self::merge(&video_path, &audio_path, &final_mp4)?;
            let _ = std::fs::remove_file(&video_path);
            let _ = std::fs::remove_file(&audio_path);
            println!("完成: {}", final_mp4);
        } else {
            // 只下载音频
            let best_audio = formats.audios.iter()
                .max_by_key(|f| f.bandwidth.unwrap_or(0))
                .context("没有可用的音频格式")?;

            let audio_path = format!("{}_audio.mp4", stem);
            let audio_out = format!("{}.m4a", stem);
            println!("已选音频: {}kbps", best_audio.bandwidth.unwrap_or(0) / 1000);
            println!("下载音频中...");
            downloader::download(&client, &best_audio.url, &audio_path).await?;
            std::fs::rename(&audio_path, &audio_out)
                .context("重命名音频文件失败")?;
            println!("完成: {}", audio_out);
        }

        Ok(())
    }

    fn merge(video: &str, audio: &str, output: &str) -> Result<()> {
        use mp4forge::mux::{MuxRequest, MuxTrackSpec, mux_to_path};
        mux_to_path(
            &MuxRequest::new(vec![
                MuxTrackSpec::path(video),
                MuxTrackSpec::path(audio),
            ]),
            output,
        ).context("mp4forge 合成失败")
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
