//! 铃声目录。前后端共用同一份定义：前端只拿 id 与名称，具体播放由平台层负责。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ringtone {
    pub id: &'static str,
    pub name_zh: &'static str,
    pub name_en: &'static str,
}

macro_rules! ringtone {
    ($id:literal, $zh:literal, $en:literal) => {
        Ringtone {
            id: $id,
            name_zh: $zh,
            name_en: $en,
        }
    };
}

static RINGTONES: [Ringtone; 10] = [
    ringtone!("glass", "玻璃", "Glass"),
    ringtone!("ping", "乒乓", "Ping"),
    ringtone!("pop", "流行", "Pop"),
    ringtone!("tink", "丁当", "Tink"),
    ringtone!("hero", "英雄", "Hero"),
    ringtone!("submarine", "潜水艇", "Submarine"),
    ringtone!("morse", "摩斯", "Morse"),
    ringtone!("purr", "呼噜", "Purr"),
    ringtone!("funk", "放克", "Funk"),
    ringtone!("bottle", "瓶子", "Bottle"),
];

pub fn all() -> &'static [Ringtone] {
    &RINGTONES
}
/// 未知 id 回退到默认铃声，保证任何配置都能响。
pub fn normalize(id: &str) -> &'static str {
    RINGTONES
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.id)
        .unwrap_or("glass")
}
