// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader for compiled SOL v8 runtime data, reconstructed from the reference executable.
//! Field names remain raw where the retained binary evidence does not establish semantics.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub diffuse: [f32; 4],
    pub ambient: [f32; 4],
    pub specular: [f32; 4],
    pub emission: [f32; 4],
    pub shininess: f32,
    pub flags: i32,
    pub texture: String,
    pub optional: Option<(i32, f32)>,
}
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub position: [f32; 3],
    pub duration: f32,
    pub links: [i32; 3],
    pub flags: i32,
    /// Quaternion stored by the reference as [w,x,y,z].
    pub rotation: Option<[f32; 4]>,
}
#[derive(Clone, Debug, Default)]
pub struct Item {
    pub position: [f32; 3],
    pub kind: i32,
    pub value: i32,
}
#[derive(Clone, Debug, Default)]
pub struct Switch {
    pub position: [f32; 3],
    pub radius: f32,
    pub path: i32,
    pub time: f32,
    pub discarded_float: f32,
    pub words: [i32; 3],
}
#[derive(Clone, Debug, Default)]
pub struct Billboard {
    pub flags: i32,
    pub material: i32,
    pub parameters: [f32; 20],
}
#[derive(Clone, Debug, Default)]
pub struct Sol {
    pub metadata: BTreeMap<String, String>,
    pub materials: Vec<Material>,
    pub vertices: Vec<[f32; 3]>,
    pub edges: Vec<[i32; 2]>,
    pub planes: Vec<[f32; 4]>,
    pub uv: Vec<[f32; 2]>,
    pub corners: Vec<[i32; 3]>,
    pub triangles: Vec<[i32; 4]>,
    pub lumps: Vec<[i32; 9]>,
    pub nodes: Vec<[i32; 5]>,
    pub paths: Vec<Path>,
    pub bodies: Vec<[i32; 7]>,
    pub items: Vec<Item>,
    pub goals: Vec<[f32; 4]>,
    pub jumps: Vec<[f32; 7]>,
    pub switches: Vec<Switch>,
    pub billboards: Vec<Billboard>,
    pub balls: Vec<[f32; 4]>,
    pub views: Vec<[f32; 6]>,
    pub indices: Vec<i32>,
    pub parsed_bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolError(pub String);
impl fmt::Display for SolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SolError {}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SolError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| SolError("size overflow".into()))?;
        let part = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| SolError(format!("truncated SOL at {}: need {} bytes", self.at, n)))?;
        self.at = end;
        Ok(part)
    }
    fn i32(&mut self) -> Result<i32, SolError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, SolError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn ints<const N: usize>(&mut self) -> Result<[i32; N], SolError> {
        let mut v = [0; N];
        for x in &mut v {
            *x = self.i32()?;
        }
        Ok(v)
    }
    fn floats<const N: usize>(&mut self) -> Result<[f32; N], SolError> {
        let mut v = [0.; N];
        for x in &mut v {
            *x = self.f32()?;
        }
        Ok(v)
    }
    fn many<T>(
        &mut self,
        count: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, SolError>,
    ) -> Result<Vec<T>, SolError> {
        (0..count).map(|_| read(self)).collect()
    }
}
fn text_at(bytes: &[u8], offset: i32) -> Result<String, SolError> {
    let tail = bytes
        .get(usize::try_from(offset).map_err(|_| SolError("negative text offset".into()))?..)
        .ok_or_else(|| SolError("text offset out of range".into()))?;
    let end = tail
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| SolError("unterminated text".into()))?;
    String::from_utf8(tail[..end].to_vec()).map_err(|_| SolError("invalid UTF-8 text".into()))
}
impl Sol {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SolError> {
        let mut r = Reader { bytes, at: 0 };
        if r.i32()? as u32 != 0x4c4f53af {
            return Err(SolError("SOL signature mismatch".into()));
        }
        if r.i32()? != 8 {
            return Err(SolError("only recorded SOL version 8 is supported".into()));
        }
        let raw = r.ints::<21>()?;
        let minimum = [
            1usize, 8, 136, 12, 8, 16, 8, 12, 16, 36, 20, 32, 28, 20, 16, 28, 40, 88, 16, 24, 4,
        ];
        let mut counts = [0usize; 21];
        let mut min_bytes = 92usize;
        for i in 0..21 {
            counts[i] =
                usize::try_from(raw[i]).map_err(|_| SolError("negative table count".into()))?;
            if counts[i] > 1_000_000 {
                return Err(SolError("table count exceeds reader limit".into()));
            }
            min_bytes = min_bytes
                .checked_add(
                    counts[i]
                        .checked_mul(minimum[i])
                        .ok_or_else(|| SolError("table size overflow".into()))?,
                )
                .ok_or_else(|| SolError("SOL size overflow".into()))?;
        }
        if min_bytes > bytes.len() {
            return Err(SolError("table sizes exceed available bytes".into()));
        }
        let text = r.take(counts[0])?;
        let dictionaries = r.many(counts[1], |r| r.ints::<2>())?;
        let mut metadata = BTreeMap::new();
        for d in dictionaries {
            metadata.insert(text_at(text, d[0])?, text_at(text, d[1])?);
        }
        let materials = r.many(counts[2], |r| {
            let diffuse = r.floats()?;
            let ambient = r.floats()?;
            let specular = r.floats()?;
            let emission = r.floats()?;
            let shininess = r.f32()?;
            let flags = r.i32()?;
            let name = r.take(64)?;
            let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            let texture = String::from_utf8(name[..len].to_vec())
                .map_err(|_| SolError("invalid material name".into()))?;
            let optional = if flags & 0x200 != 0 {
                Some((r.i32()?, r.f32()?))
            } else {
                None
            };
            Ok(Material {
                diffuse,
                ambient,
                specular,
                emission,
                shininess,
                flags,
                texture,
                optional,
            })
        })?;
        let vertices = r.many(counts[3], |r| r.floats())?;
        let edges = r.many(counts[4], |r| r.ints())?;
        let planes = r.many(counts[5], |r| r.floats())?;
        let uv = r.many(counts[6], |r| r.floats())?;
        let corners = r.many(counts[7], |r| r.ints())?;
        let triangles = r.many(counts[8], |r| r.ints())?;
        let lumps = r.many(counts[9], |r| r.ints())?;
        let nodes = r.many(counts[10], |r| r.ints())?;
        let paths = r.many(counts[11], |r| {
            let position = r.floats()?;
            let duration = r.f32()?;
            let links = r.ints()?;
            let flags = r.i32()?;
            let rotation = if flags & 1 != 0 {
                Some(r.floats()?)
            } else {
                None
            };
            Ok(Path {
                position,
                duration,
                links,
                flags,
                rotation,
            })
        })?;
        let bodies = r.many(counts[12], |r| r.ints())?;
        let items = r.many(counts[13], |r| {
            Ok(Item {
                position: r.floats()?,
                kind: r.i32()?,
                value: r.i32()?,
            })
        })?;
        let goals = r.many(counts[14], |r| r.floats())?;
        let jumps = r.many(counts[15], |r| r.floats())?;
        let switches = r.many(counts[16], |r| {
            Ok(Switch {
                position: r.floats()?,
                radius: r.f32()?,
                path: r.i32()?,
                time: r.f32()?,
                discarded_float: r.f32()?,
                words: r.ints()?,
            })
        })?;
        let billboards = r.many(counts[17], |r| {
            Ok(Billboard {
                flags: r.i32()?,
                material: r.i32()?,
                parameters: r.floats()?,
            })
        })?;
        let balls = r.many(counts[18], |r| r.floats())?;
        let views = r.many(counts[19], |r| r.floats())?;
        let indices = r.many(counts[20], |r| r.i32())?;
        if r.at != bytes.len() {
            return Err(SolError(format!("{} trailing bytes", bytes.len() - r.at)));
        }
        let result = Self {
            metadata,
            materials,
            vertices,
            edges,
            planes,
            uv,
            corners,
            triangles,
            lumps,
            nodes,
            paths,
            bodies,
            items,
            goals,
            jumps,
            switches,
            billboards,
            balls,
            views,
            indices,
            parsed_bytes: r.at,
        };
        result.check_references()?;
        Ok(result)
    }
    fn check_references(&self) -> Result<(), SolError> {
        fn index(v: i32, len: usize, what: &str) -> Result<(), SolError> {
            if v < 0 || v as usize >= len {
                Err(SolError(format!("{} index {} outside {}", what, v, len)))
            } else {
                Ok(())
            }
        }
        for edge in &self.edges {
            for &v in edge {
                index(v, self.vertices.len(), "edge vertex")?;
            }
        }
        for corner in &self.corners {
            index(corner[0], self.uv.len(), "corner UV")?;
            index(corner[1], self.planes.len(), "corner normal")?;
            index(corner[2], self.vertices.len(), "corner vertex")?;
        }
        for triangle in &self.triangles {
            index(triangle[0], self.materials.len(), "triangle material")?;
            for &c in &triangle[1..] {
                index(c, self.corners.len(), "triangle corner")?;
            }
        }
        for path in &self.paths {
            if path.links[0] >= 0 {
                index(path.links[0], self.paths.len(), "next path")?;
            }
        }
        for b in &self.bodies {
            for &p in &b[..2] {
                if p >= 0 {
                    index(p, self.paths.len(), "body path")?;
                }
            }
        }
        for s in &self.switches {
            if s.path >= 0 {
                index(s.path, self.paths.len(), "switch path")?;
            }
        }
        for b in &self.billboards {
            index(b.material, self.materials.len(), "billboard material")?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(0x4c4f53afu32.to_le_bytes());
        b.extend(8i32.to_le_bytes());
        b.extend([0u8; 84]);
        b
    }
    #[test]
    fn empty_and_truncated_files() {
        let b = empty();
        assert_eq!(Sol::from_bytes(&b).unwrap().parsed_bytes, 92);
        for n in 0..b.len() {
            assert!(Sol::from_bytes(&b[..n]).is_err());
        }
        let mut trailing = b.clone();
        trailing.push(0);
        assert!(Sol::from_bytes(&trailing).is_err());
    }
    #[test]
    fn counts_and_signature_are_bounded() {
        let mut b = empty();
        b[0] = 0;
        assert!(Sol::from_bytes(&b).is_err());
        let mut b = empty();
        b[8..12].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(Sol::from_bytes(&b).is_err());
        let mut b = empty();
        b[8..12].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(Sol::from_bytes(&b).is_err());
    }
}
