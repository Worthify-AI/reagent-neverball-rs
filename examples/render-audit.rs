use reagent_neverball_rs::sol::Sol;
use std::collections::BTreeSet;
fn main() {
    for path in std::env::args().skip(1) {
        let s = Sol::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
        let mut all = BTreeSet::new();
        println!("{path}: {} triangles", s.triangles.len());
        for (i, b) in s.bodies.iter().enumerate() {
            let mut ids = BTreeSet::new();
            ids.extend(
                s.indices
                    .iter()
                    .skip(b[5].max(0) as usize)
                    .take(b[6].max(0) as usize)
                    .copied(),
            );
            for l in s
                .lumps
                .iter()
                .skip(b[3].max(0) as usize)
                .take(b[4].max(0) as usize)
            {
                ids.extend(
                    s.indices
                        .iter()
                        .skip(l[5].max(0) as usize)
                        .take(l[6].max(0) as usize)
                        .copied(),
                );
            }
            println!("body{i}: {b:?}, triangles{}", ids.len());
            all.extend(ids);
        }
        println!("unique {}, paths {:?}", all.len(), s.paths);
    }
}
