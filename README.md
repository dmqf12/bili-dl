# bili-dl

纯 Rust 实现的 B站视频下载器，零外部依赖（无需 ffmpeg）。

## 特性

- 🦀 纯 Rust，无需安装 ffmpeg
- 🎵 支持仅下载音频/视频
- 📦 可选自动合成或保留分离文件
- 🍪 支持 Cookie 登录
- 🔗 支持 BV号、完整URL、短链接

## 安装

```bash
cargo install bili-dl
```

## CLI 使用

```bash
# 基础下载
bili-dl BV1EXgz6iE7N
bili-dl https://www.bilibili.com/video/BV1EXgz6iE7N
bili-dl https://b23.tv/TsQPRlN

# 指定输出
bili-dl BVxxx -o 视频.mp4
bili-dl BVxxx -o ./downloads/

# 仅下载音频
bili-dl BVxxx --audio-only -o ./music/

# 仅下载视频（无音频）
bili-dl BVxxx --video-only

# 不合成，保留分离的音视频文件
bili-dl BVxxx --no-merge

# 使用 Cookie
bili-dl BVxxx --cookie cookies.json
```

## API 使用

```rust
use bili_dl::BiliDownloader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 完整下载
    BiliDownloader::new("BV1EXgz6iE7N")
        .download()
        .await?;

    // 仅下载音频
    BiliDownloader::new("https://www.bilibili.com/video/BV1EXgz6iE7N")
        .audio_only(true)
        .output("./music/")
        .download()
        .await?;

    // 仅下载视频
    BiliDownloader::new("BV1EXgz6iE7N")
        .video_only(true)
        .output("output.mp4")
        .download()
        .await?;

    // 不合成
    BiliDownloader::new("BV1EXgz6iE7N")
        .no_merge(true)
        .download()
        .await?;

    Ok(())
}
```

## 选项

| CLI 参数 | API 方法 | 说明 |
|----------|----------|------|
| `<URL>` | `BiliDownloader::new(url)` | B站链接或 BV 号 |
| `-o, --output <路径>` | `.output(path)` | 输出文件或目录 |
| `--cookie <文件>` | `.cookie(path)` | Cookie 文件路径 |
| `--video-only` | `.video_only(true)` | 仅下载视频流 |
| `--audio-only` | `.audio_only(true)` | 仅下载音频流 |
| `--no-merge` | `.no_merge(true)` | 不合成音视频 |

## 许可

LGPL-2.1-only
