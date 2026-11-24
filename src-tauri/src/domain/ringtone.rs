//! 铃声目录的"约定"。
//!
//! 一个文件夹就是一首铃声，文件夹名就是列表里显示的名字：
//!
//! ```text
//!   <data>/ringtones/
//!     emotional-piano/
//!       nastelbom-...-clip.m4a        ← 有 m4a / mp3 才算一首铃声
//!       ...-license.txt               ← 有 txt 就当版权声明
//!     love-story/
//!     my-own-ringtone/                ← 用户自己加的，也是同样的结构
//! ```
//!
//! 这样铃声库对用户是完全透明的：不喜欢的删掉文件夹、想换曲子直接丢个 m4a 进去，
//! 应用启动时扫一遍就生效，不需要额外的索引文件。
//!
//! 本模块只放纯逻辑（判断扩展名、清洗文件夹名），扫盘与复制在 `infra::ringtones`。

use std::path::Path;

use serde::Serialize;

/// 前端列表 / 闹钟配置里的一首铃声。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ringtone {
    /// 文件夹名，同时也是闹钟里存的 `ringtoneIn` / `ringtoneOut`。
    pub id: String,
    /// 显示名，就是文件夹名。
    pub name: String,
    /// 文件夹里是否有版权声明。
    pub has_license: bool,
}

/// 认得的音频扩展名：只认这两种，避免把封面图、说明文件当成铃声。
const AUDIO_EXTENSIONS: [&str; 2] = ["m4a", "mp3"];
/// 版权声明扩展名。
const LICENSE_EXTENSION: &str = "txt";

fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

pub fn is_audio(path: &Path) -> bool {
    extension_lower(path).is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.as_str()))
}

pub fn is_license(path: &Path) -> bool {
    extension_lower(path).as_deref() == Some(LICENSE_EXTENSION)
}

/// 文件夹名里不允许出现的字符（跨平台都要安全）。
const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
/// 名字最长多少个字符，太长的名字在列表里也会被截断。
const MAX_NAME_CHARS: usize = 48;

/// 把用户输入的"音乐名"整理成可用的文件夹名。
///
/// 非法字符换成 `-`，首尾空格和点去掉（Windows 不允许以点结尾），
/// 空名或以点开头的名字直接拒绝——那会变成隐藏文件夹，用户在访达里看不见。
pub fn folder_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if FORBIDDEN.contains(&ch) || ch.is_control() {
                '-'
            } else {
                ch
            }
        })
        .collect();

    let trimmed = cleaned.trim_matches(|ch: char| ch == ' ' || ch == '.');
    let name: String = trimmed.chars().take(MAX_NAME_CHARS).collect();
    let name = name.trim_end().to_string();

    if name.is_empty() || name.starts_with('.') {
        return None;
    }
    Some(name)
}

/// 老配置里的铃声 id 可能已经不存在（用户删了文件夹，或升级前的内置音效），
/// 这时回退到列表第一首，保证闹钟一定响得出来。
pub fn normalize<'a>(id: &str, available: &'a [Ringtone]) -> Option<&'a str> {
    available
        .iter()
        .find(|item| item.id == id)
        .or_else(|| available.first())
        .map(|item| item.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(id: &str) -> Ringtone {
        Ringtone {
            id: id.to_string(),
            name: id.to_string(),
            has_license: false,
        }
    }

    #[test]
    fn 只认_m4a_和_mp3() {
        assert!(is_audio(Path::new("/a/b.M4A")));
        assert!(is_audio(Path::new("/a/b.mp3")));
        assert!(!is_audio(Path::new("/a/b.wav")));
        assert!(!is_audio(Path::new("/a/b.txt")));
        assert!(!is_audio(Path::new("/a/b")));
    }

    #[test]
    fn 版权文件只认_txt() {
        assert!(is_license(Path::new("/a/license.TXT")));
        assert!(is_license(Path::new("/a/license.txt")));
        assert!(!is_license(Path::new("/a/license.pdf")));
    }

    #[test]
    fn 文件夹名会被清洗() {
        assert_eq!(folder_name("  我的铃声  ").as_deref(), Some("我的铃声"));
        assert_eq!(folder_name("a/b:c").as_deref(), Some("a-b-c"));
        // 首尾的点会被去掉，否则在访达里看不见
        assert_eq!(folder_name("..hidden").as_deref(), Some("hidden"));
        assert_eq!(folder_name("   "), None);
        assert_eq!(folder_name("..."), None);
    }

    #[test]
    fn 名字不会超长() {
        let long = "长".repeat(120);
        let name = folder_name(&long).unwrap();
        assert_eq!(name.chars().count(), MAX_NAME_CHARS);
    }

    #[test]
    fn 找不到就回退到第一首() {
        let list = vec![tone("a"), tone("b")];
        assert_eq!(normalize("b", &list), Some("b"));
        assert_eq!(normalize("gone", &list), Some("a"));
        assert_eq!(normalize("", &[]), None);
    }
}
