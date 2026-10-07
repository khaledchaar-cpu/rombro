//! Firmware recognised by size + SHA1 against libretro's bundled `System.dat`.

use rombro_core::Hashes;
use rombro_core::plan::{Game, Ident};
use rombro_core::retroarch::firmware;
use std::collections::HashMap;
use std::sync::OnceLock;

/// SHA1 → (size, every path the file is listed under).
type Index = HashMap<[u8; 20], (u64, Vec<Game>)>;

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut idx: Index = HashMap::new();
        for f in firmware::bundled() {
            let (_, games) = idx.entry(f.sha1).or_insert((f.size, Vec::new()));
            // the same file may be listed under several systems at one path
            if !games.iter().any(|g| g.name == f.path) {
                games.push(Game {
                    system: f.system,
                    name: f.path,
                    crc: None,
                });
            }
        }
        idx
    })
}

/// `Ident::Firmware` if `h` is exactly a known firmware file (size and SHA1).
pub(crate) fn ident(h: &Hashes) -> Option<Ident> {
    let (size, games) = index().get(&h.sha1)?;
    (*size == h.size).then(|| Ident::Firmware(games.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_size_and_sha1_only() {
        let f = &firmware::bundled()[0];
        let h = Hashes {
            size: f.size,
            crc: 0,
            sha1: f.sha1,
            md5: None,
        };
        let Some(Ident::Firmware(g)) = ident(&h) else {
            panic!("not recognised");
        };
        assert!(g.iter().any(|g| g.name == f.path));
        assert!(ident(&Hashes { size: 1, ..h }).is_none());
    }
}
