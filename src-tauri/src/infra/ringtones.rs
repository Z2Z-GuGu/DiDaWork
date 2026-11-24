//! 铃声库的磁盘操作：扫描、首次复制内置铃声、导入用户自定义铃声。
//!
//! 目录结构见 `domain::ringtone`。这里不缓存任何索引——每次 `list()` 直接扫盘，
//! 用户在外面往文件夹里丢一首 m4a，回到应用就能看到，不需要重启或刷新按钮。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::ringtone::{self, Ringtone};

/// 数据目录下的子目录名。安装后内置铃声就复制到这里，用户自定义的也放这里。
pub const DIR_NAME: &str = "ringtones";
/// 内置铃声只复制一次；用户删掉的不会被下次启动偷偷补回来。
const SEED_MARKER: &str = ".seeded";

/// 导入失败的原因。`code` 给前端选文案，`message` 兜底展示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RingtoneError {
    pub code: String,
    pub message: String,
}

impl RingtoneError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }

    fn io(err: io::Error) -> Self {
        Self::new("io", err.to_string())
    }
}

pub struct Library {
    root: PathBuf,
}

impl Library {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }

    /// 首次启动把安装包里的内置铃声复制到用户目录，返回复制的目录数。
    ///
    /// 只看 `.seeded` 标记，不看目录是否存在：否则用户在访达里删掉不喜欢的铃声，
    /// 下次启动又会长回来，很烦人。
    pub fn seed(&self, source: &Path) -> io::Result<usize> {
        self.ensure()?;
        let marker = self.root.join(SEED_MARKER);
        if marker.exists() || !source.is_dir() {
            return Ok(0);
        }

        let mut copied = 0;
        for entry in fs::read_dir(source)?.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let target = self.root.join(entry.file_name());
            if target.exists() {
                continue;
            }
            if copy_tree(&path, &target).is_ok() {
                copied += 1;
            }
        }
        // 标记只在整轮结束后写；中途失败下次启动会重试，已复制的目录因为存在而被跳过。
        fs::write(&marker, b"1")?;
        Ok(copied)
    }

    /// 扫描全部铃声，按文件名排序（不区分大小写）。
    pub fn list(&self) -> Vec<Ringtone> {
        let mut items = Vec::new();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return items;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let Some((audio, license)) = scan(&path) else {
                continue;
            };
            let _ = audio;
            items.push(Ringtone {
                id: name.clone(),
                name,
                has_license: license.is_some(),
            });
        }
        items.sort_by_key(|item| item.name.to_lowercase());
        items
    }

    /// 某首铃声的音频文件。路径全部由我们自己拼，仍做一次校验挡掉 `../` 之类的 id。
    pub fn audio_path(&self, id: &str) -> Option<PathBuf> {
        let dir = self.dir_of(id)?;
        scan(&dir).map(|(audio, _)| audio)
    }

    pub fn license_path(&self, id: &str) -> Option<PathBuf> {
        let dir = self.dir_of(id)?;
        scan(&dir).and_then(|(_, license)| license)
    }

    /// 第一首可用铃声的音频，用于"配置里的铃声已经不存在"时兜底。
    pub fn first_audio(&self) -> Option<PathBuf> {
        self.list()
            .first()
            .and_then(|item| self.audio_path(&item.id))
    }

    /// 导入一首自定义铃声：新建文件夹 + 复制音频 + 可选复制版权声明。
    pub fn create(
        &self,
        name: &str,
        audio: &Path,
        license: Option<&Path>,
    ) -> Result<Ringtone, RingtoneError> {
        let Some(folder) = ringtone::folder_name(name) else {
            return Err(RingtoneError::new("nameInvalid", "音乐名不可用"));
        };
        if !audio.is_file() {
            return Err(RingtoneError::new("audioInvalid", "找不到选中的音频文件"));
        }
        if !ringtone::is_audio(audio) {
            return Err(RingtoneError::new("audioInvalid", "只支持 .m4a / .mp3"));
        }
        if let Some(path) = license {
            if !path.is_file() || !ringtone::is_license(path) {
                return Err(RingtoneError::new("licenseInvalid", "版权声明需为 .txt"));
            }
        }

        let target = self.root.join(&folder);
        if target.exists() {
            return Err(RingtoneError::new("nameTaken", format!("“{folder}”已存在")));
        }

        self.ensure().map_err(RingtoneError::io)?;
        fs::create_dir_all(&target).map_err(RingtoneError::io)?;

        // 复制过程中出错就把半成品目录删掉，别在列表里留下一个空壳。
        let result = (|| -> io::Result<()> {
            copy_file(audio, &target)?;
            if let Some(path) = license {
                copy_file(path, &target)?;
            }
            Ok(())
        })();
        if let Err(err) = result {
            let _ = fs::remove_dir_all(&target);
            return Err(RingtoneError::io(err));
        }

        Ok(Ringtone {
            id: folder.clone(),
            name: folder,
            has_license: license.is_some(),
        })
    }

    fn dir_of(&self, id: &str) -> Option<PathBuf> {
        if id.is_empty() || id.starts_with('.') || id.contains(['/', '\\']) {
            return None;
        }
        let dir = self.root.join(id);
        dir.is_dir().then_some(dir)
    }
}

/// 文件夹里挑出音频与版权声明：各取文件名排序后的第一个。
fn scan(dir: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
    let mut audio: Vec<PathBuf> = Vec::new();
    let mut licenses: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if ringtone::is_audio(&path) {
            audio.push(path);
        } else if ringtone::is_license(&path) {
            licenses.push(path);
        }
    }
    audio.sort();
    licenses.sort();
    audio.into_iter().next().map(|first| (first, licenses.into_iter().next()))
}

fn copy_file(from: &Path, dir: &Path) -> io::Result<PathBuf> {
    let name = from
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "文件名无效"))?;
    let target = dir.join(name);
    fs::copy(from, &target)?;
    Ok(target)
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name.to_str() == Some(".DS_Store") {
            continue;
        }
        let target = to.join(&name);
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_lib(tag: &str) -> (Library, PathBuf) {
        let root = std::env::temp_dir().join(format!("didawork-ringtones-{tag}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        (Library::new(&root), root)
    }

    fn write(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content).unwrap();
    }

    #[test]
    fn 扫描时只认有音频的文件夹() {
        let (lib, root) = temp_lib("scan");
        write(&root.join("a/one.m4a"), b"x");
        write(&root.join("a/license.txt"), b"x");
        write(&root.join("b/cover.png"), b"x");
        write(&root.join("c/two.mp3"), b"x");

        let list = lib.list();
        let ids: Vec<_> = list.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c"]);
        assert!(list[0].has_license);
        assert!(!list[1].has_license);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn 导入后立刻出现在列表里() {
        let (lib, root) = temp_lib("create");
        let src_audio = root.join("src.mp3");
        write(&src_audio, b"audio");
        let src_license = root.join("src-license.txt");
        write(&src_license, b"license");

        let created = lib.create("我的铃声", &src_audio, Some(&src_license)).unwrap();
        assert_eq!(created.id, "我的铃声");
        assert!(created.has_license);
        assert!(lib.audio_path("我的铃声").is_some());
        assert!(lib.license_path("我的铃声").unwrap().ends_with("src-license.txt"));
        // 列表里能扫到它
        assert_eq!(lib.list().len(), 1);

        // 同名再来一次要被拒绝
        let err = lib.create("我的铃声", &src_audio, None).unwrap_err();
        assert_eq!(err.code, "nameTaken");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn 拒绝非音频与非法名字() {
        let (lib, root) = temp_lib("invalid");
        let wav = root.join("a.wav");
        write(&wav, b"x");
        assert_eq!(lib.create("x", &wav, None).unwrap_err().code, "audioInvalid");
        assert_eq!(lib.create("  ", &wav, None).unwrap_err().code, "nameInvalid");

        let mp3 = root.join("b.mp3");
        write(&mp3, b"x");
        let pdf = root.join("license.pdf");
        write(&pdf, b"x");
        assert_eq!(
            lib.create("ok", &mp3, Some(&pdf)).unwrap_err().code,
            "licenseInvalid"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn 内置铃声只复制一次() {
        let bundled = std::env::temp_dir().join("didawork-ringtones-bundled");
        let _ = fs::remove_dir_all(&bundled);
        write(&bundled.join("emotional-piano/a.m4a"), b"x");
        write(&bundled.join("emotional-piano/license.txt"), b"x");
        write(&bundled.join("medical/a.m4a"), b"x");

        let (lib, root) = temp_lib("seed");
        assert_eq!(lib.seed(&bundled).unwrap(), 2);
        assert_eq!(lib.list().len(), 2);
        // 用户删掉一首，再启动不该补回来
        fs::remove_dir_all(root.join("medical")).unwrap();
        assert_eq!(lib.seed(&bundled).unwrap(), 0);
        assert_eq!(lib.list().len(), 1);

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&bundled);
    }

    #[test]
    fn 路径穿越的_id_取不到文件() {
        let (lib, root) = temp_lib("escape");
        write(&root.join("a/one.mp3"), b"x");
        assert!(lib.audio_path("../a").is_none());
        assert!(lib.audio_path("..").is_none());
        assert!(lib.audio_path("a/").is_none());
        let _ = fs::remove_dir_all(root);
    }
}
