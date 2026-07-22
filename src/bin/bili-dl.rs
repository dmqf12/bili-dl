use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let url = &args[1];
    if url == "-h" || url == "--help" {
        print_usage();
        return Ok(());
    }

    let mut output_name = None;
    let mut cookie_path = None;
    let mut video_only = false;
    let mut audio_only = false;
    let mut no_merge = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output_name = Some(args.get(i).context("-o 需要指定文件名或目录")?.clone());
            }
            "--cookie" => {
                i += 1;
                cookie_path = Some(args.get(i).context("--cookie 需要指定文件路径")?.clone());
            }
            "--video-only" => video_only = true,
            "--audio-only" => audio_only = true,
            "--no-merge" => no_merge = true,
            _ => {
                anyhow::bail!("未知参数: {}", args[i]);
            }
        }
        i += 1;
    }

    if video_only && audio_only {
        anyhow::bail!("--video-only 和 --audio-only 不能同时使用");
    }

    let mut dl = bili_dl::BiliDownloader::new(url)
        .video_only(video_only)
        .audio_only(audio_only)
        .no_merge(no_merge);

    if let Some(ref p) = cookie_path {
        dl = dl.cookie(p);
    }
    if let Some(ref o) = output_name {
        dl = dl.output(o);
    }

    dl.download().await
}

fn print_usage() {
    eprintln!("bili-dl 2.0 —— B站视频下载器 (纯Rust)");
    eprintln!();
    eprintln!("用法:");
    eprintln!("  bili-dl <URL> [-o <路径>] [--cookie <文件>] [--video-only] [--audio-only] [--no-merge]");
    eprintln!("  bili-dl <BV号> [-o <路径>] [--cookie <文件>] [--video-only] [--audio-only] [--no-merge]");
    eprintln!();
    eprintln!("选项:");
    eprintln!("  -o, --output <路径>  输出文件或目录（目录需以 / 结尾）");
    eprintln!("  --cookie <文件>      登录 cookie（JSON 或 Netscape 格式）");
    eprintln!("  --video-only         只下载视频流");
    eprintln!("  --audio-only         只下载音频流");
    eprintln!("  --no-merge           不合成，保留分离的音视频文件");
    eprintln!("  -h, --help           显示此帮助");
    eprintln!();
    eprintln!("示例:");
    eprintln!("  bili-dl BVxxx");
    eprintln!("  bili-dl https://www.bilibili.com/video/BVxxx");
    eprintln!("  bili-dl BVxxx -o 视频.mp4");
    eprintln!("  bili-dl BVxxx --audio-only -o ./music/");
    eprintln!("  bili-dl BVxxx --no-merge");
}
