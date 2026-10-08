use reagent_neverball_rs::{content::Catalog, sol::Sol};
use std::{env, fs, path::Path};
fn walk(p: &Path, paths: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(p).unwrap() {
        let e = e.unwrap().path();
        if e.is_dir() {
            walk(&e, paths)
        } else if e.extension().is_some_and(|x| x == "sol") {
            paths.push(e)
        }
    }
}
fn main() {
    let root = env::args().nth(1).expect("runtime data directory");
    let root = Path::new(&root);
    let catalog = Catalog::parse(&fs::read_to_string(root.join("sets.txt")).unwrap(), |p| {
        fs::read_to_string(root.join(p)).map_err(|e| e.to_string())
    })
    .unwrap();
    let mut paths = Vec::new();
    walk(root, &mut paths);
    let mut total = 0;
    for p in &paths {
        let b = fs::read(p).unwrap();
        let sol = Sol::from_bytes(&b).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
        assert_eq!(sol.parsed_bytes, b.len());
        total += b.len();
    }
    println!(
        "sets={} gameplay_levels={} sol_files={} parsed_bytes={}",
        catalog.sets.len(),
        catalog.level_count(),
        paths.len(),
        total
    );
}
