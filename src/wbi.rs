use anyhow::{Context, Result};
use md5::{Digest, Md5};
use reqwest::Client;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// WBI 签名密钥
#[derive(Clone, Debug)]
pub struct WbiKey {
    key: String,
}

impl WbiKey {
    /// 对参数进行 WBI 签名，返回带签名的参数字典
    pub fn sign(&self, params: &mut Vec<(String, String)>) -> HashMap<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        params.push(("wts".to_string(), now.to_string()));

        params.sort_by(|a, b| a.0.cmp(&b.0));

        let mut result: HashMap<String, String> = HashMap::new();
        let mut query_parts: Vec<String> = Vec::new();

        for (k, v) in params.iter() {
            let filtered: String = v.chars().filter(|c| !"!'()*".contains(*c)).collect();
            result.insert(k.clone(), filtered.clone());
            query_parts.push(format!("{}={}", k, filtered));
        }

        let query_string = query_parts.join("&");
        let to_hash = format!("{}{}", query_string, self.key);
        let mut hasher = Md5::new();
        hasher.update(to_hash.as_bytes());
        let hash_result = hasher.finalize();
        let w_rid: String = hash_result
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        result.insert("w_rid".to_string(), w_rid);
        result
    }
}

/// mixin key 置换表 (来自 B站前端 JS getMixinKey)
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
    37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

/// 从 nav API 获取 WBI 密钥
pub async fn get_wbi_key(client: &Client) -> Result<WbiKey> {
    let nav = crate::api::get_nav(client).await?;

    let img_url = nav["data"]["wbi_img"]["img_url"]
        .as_str()
        .context("无法获取 wbi img_url")?;
    let sub_url = nav["data"]["wbi_img"]["sub_url"]
        .as_str()
        .context("无法获取 wbi sub_url")?;

    let img_key = extract_key_from_url(img_url);
    let sub_key = extract_key_from_url(sub_url);
    let lookup = format!("{}{}", img_key, sub_key);

    let key: String = MIXIN_KEY_ENC_TAB
        .iter()
        .filter_map(|&i| lookup.chars().nth(i))
        .take(32)
        .collect();

    Ok(WbiKey { key })
}

/// 从 wbi URL 提取 key（取文件名，去掉扩展名）
fn extract_key_from_url(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
        .split('.').next().unwrap_or(url)
}
