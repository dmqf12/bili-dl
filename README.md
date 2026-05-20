使用deepseek参照yt-dlp基于rust构建的b站视频下载器
bili-dl —— B站视频下载器 (Rust)

用法:
  bili-dl <URL> [-o <路径>] [--cookie <文件>]
  bili-dl <BV号> [-o <路径>] [--cookie <文件>]

选项:
  -o, --output <路径>  输出文件或目录（目录需以 / 结尾，文件名用视频标题）
  --cookie <文件>      登录 cookie（JSON 或 Netscape 格式）
  -h, --help           显示此帮助

示例:
  bili-dl BVxxx
  bili-dl https://www.bilibili.com/video/BVxxx
  bili-dl https://www.bilibili.com/video/BVxxx -o 视频.mp4
  bili-dl https://www.bilibili.com/video/BVxxx -o ./downloads/ --cookie cookies.json

或者作为库调用
bili_dl::BiliDownloader::new("链接或者bv号").output().cookie().download().await;
