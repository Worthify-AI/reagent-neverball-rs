// SPDX-License-Identifier: GPL-3.0-or-later
//! Packaged set manifests and metadata; no engine implementation source is used.
use crate::sol::Sol;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelSet {
    pub file: String,
    pub title: String,
    pub description: String,
    pub id: String,
    pub preview: String,
    pub records: [i32; 6],
    pub levels: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub sets: Vec<LevelSet>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelSpec {
    pub path: String,
    pub title: String,
    pub message: String,
    pub author: String,
    pub background: String,
    pub gradient: String,
    pub screenshot: String,
    pub song: String,
    pub time_centiseconds: i32,
    pub goal_coins: i32,
    pub bonus: bool,
    pub time_records: Vec<i32>,
    pub coin_records: Vec<i32>,
    pub goal_records: Vec<i32>,
}
/// Browser catalogue generated from the same packaged SOL metadata as native menus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelMetadataIndex {
    pub version: u32,
    pub catalog: Catalog,
    pub levels: Vec<Vec<LevelSpec>>,
}
impl LevelMetadataIndex {
    pub fn generate(
        catalog: Catalog,
        mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Self, String> {
        let levels = catalog
            .sets
            .iter()
            .map(|set| {
                set.levels
                    .iter()
                    .map(|path| {
                        let sol =
                            Sol::from_bytes(&read(path)?).map_err(|e| format!("{path}: {e}"))?;
                        Ok(LevelSpec::from_sol(path, &sol))
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            version: 1,
            catalog,
            levels,
        })
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let index: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if index.version != 1
            || index.catalog.sets.is_empty()
            || index.catalog.sets.len() != index.levels.len()
            || index
                .catalog
                .sets
                .iter()
                .zip(&index.levels)
                .any(|(set, levels)| {
                    set.levels.len() != levels.len()
                        || set
                            .levels
                            .iter()
                            .zip(levels)
                            .any(|(path, spec)| !safe_path(path) || path != &spec.path)
                })
        {
            return Err("incompatible level metadata index".into());
        }
        Ok(index)
    }
}
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != ".." && p != ".")
}
fn numbers(s: &str) -> Result<Vec<i32>, String> {
    s.split_whitespace()
        .map(|x| {
            x.parse()
                .map_err(|_| format!("invalid manifest number: {x}"))
        })
        .collect()
}
impl LevelSet {
    pub fn parse(file: &str, text: &str) -> Result<Self, String> {
        if !safe_path(file) {
            return Err("invalid set path".into());
        }
        let lines: Vec<_> = text.lines().map(str::trim).collect();
        if lines.len() < 6 {
            return Err(format!("{file}: incomplete set manifest"));
        }
        let records: [i32; 6] = if lines[4].is_empty() {
            [0; 6]
        } else {
            numbers(lines[4])?
                .try_into()
                .map_err(|_| format!("{file}: expected six set record values"))?
        };
        let levels: Vec<String> = lines[5..]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if !safe_path(lines[3]) || levels.iter().any(|p| !safe_path(p) || !p.ends_with(".sol")) {
            return Err(format!("{file}: invalid runtime data path"));
        }
        Ok(Self {
            file: file.into(),
            title: lines[0].into(),
            description: lines[1].replace('\\', "\n"),
            id: lines[2].into(),
            preview: lines[3].into(),
            records,
            levels,
        })
    }
}
impl Catalog {
    pub fn parse(
        sets: &str,
        mut read: impl FnMut(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let mut result = Self::default();
        for file in sets.lines().map(str::trim).filter(|s| !s.is_empty()) {
            result.sets.push(LevelSet::parse(file, &read(file)?)?);
        }
        if result.sets.is_empty() {
            return Err("no level sets".into());
        }
        Ok(result)
    }
    pub fn level_count(&self) -> usize {
        self.sets.iter().map(|s| s.levels.len()).sum()
    }
}
impl LevelSpec {
    pub fn from_sol(path: &str, sol: &Sol) -> Self {
        let value = |key: &str| sol.metadata.get(key).cloned().unwrap_or_default();
        let title = value("name");
        Self {
            path: path.into(),
            title: if title.is_empty() {
                path.rsplit('/')
                    .next()
                    .unwrap_or(path)
                    .trim_end_matches(".sol")
                    .replace('_', " ")
            } else {
                title
            },
            message: value("message").replace('\\', "\n"),
            author: value("author"),
            background: value("back"),
            gradient: value("grad"),
            screenshot: value("shot"),
            song: value("song"),
            time_centiseconds: value("time").parse().unwrap_or(0),
            goal_coins: value("goal").parse().unwrap_or(0),
            bonus: value("bonus") == "1",
            time_records: numbers(&value("time_hs")).unwrap_or_default(),
            coin_records: numbers(&value("coin_hs")).unwrap_or_default(),
            goal_records: numbers(&value("goal_hs")).unwrap_or_default(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifests_and_paths() {
        let data="Neverball Easy\nFirst\\Second\neasy\nshot/easy.jpg\n1 2 3 4 5 6\nmap/easy.sol\nmap/next.sol\n";
        let c = Catalog::parse("set-easy.txt\n", |_| Ok(data.into())).unwrap();
        assert_eq!(c.level_count(), 2);
        assert_eq!(c.sets[0].description, "First\nSecond");
        assert!(LevelSet::parse("set.txt", &data.replace("map/next.sol", "../next.sol")).is_err());
        for p in ["../secret", "/secret", "a/../b", "a\\b", "a//b"] {
            assert!(!safe_path(p));
        }
    }
}
