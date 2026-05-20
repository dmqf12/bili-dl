# bili-dl —— B站视频下载器 (Rust)

基于 Rust 构建的 B 站视频下载器，核心设计参照 `yt-dlp` 的使用逻辑。

## 🚀 命令行用法 (CLI)

```bash
bili-dl <URL> [-o <路径>] [--cookie <文件>]
bili-dl <BV号> [-o <路径>] [--cookie <文件>]

```

### 📋 选项说明

| 选项 | 全称 | 描述 |
| --- | --- | --- |
| `-o` | `--output <路径>` | 输出文件或目录（目录需以 `/` 结尾，留空则默认使用视频标题作为文件名） |
|  | `--cookie <文件>` | 登录 cookie 文件路径（支持 JSON 或 Netscape 格式） |
| `-h` | `--help` | 显示帮助信息 |

### 💡 使用示例

```bash
# 1. 基础下载（使用 BV 号）
bili-dl BVxxx

# 2. 基础下载（使用完整视频链接）
bili-dl https://www.bilibili.com/video/BVxxx

# 3. 指定输出文件名
bili-dl https://www.bilibili.com/video/BVxxx -o 视频.mp4

# 4. 指定下载目录并携带 Cookie 凭证
bili-dl https://www.bilibili.com/video/BVxxx -o ./downloads/ --cookie cookies.json

```

---

## 📦 作为 Rust 库 (SDK) 调用

你也可以直接将 `bili-dl` 作为依赖引入到你的 Rust 项目中，通过链式调用来执行下载任务：

```rust
use bili_dl::BiliDownloader;

#[tokio::main]
async fn main() {
    BiliDownloader::new("链接或者bv号")
        .output("./downloads/")
        .cookie("cookies.json")
        .download()
        .await;
}

```
