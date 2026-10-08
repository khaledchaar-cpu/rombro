//! Launcher data: the core chosen for a single game (setting `core:<path>`).

use crate::files::key;
use crate::{Result, Store};
use std::path::Path;

fn override_key(game: &Path) -> String {
    format!("core:{}", key(game))
}

impl Store {
    /// Core id the user chose for `game`, overriding the system's core.
    pub fn core_override(&self, game: &Path) -> Result<Option<String>> {
        self.setting(&override_key(game))
    }

    /// Sets (`Some`) or clears (`None`) the core override of `game`.
    pub fn set_core_override(&self, game: &Path, core: Option<&str>) -> Result<()> {
        match core {
            Some(c) => self.set_setting(&override_key(game), c),
            None => {
                self.conn
                    .execute("DELETE FROM settings WHERE key = ?1", [override_key(game)])?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_clears_core_override() {
        let s = Store::open_in_memory().unwrap();
        let g = Path::new("/lib/NES/a.nes");
        assert_eq!(s.core_override(g).unwrap(), None);
        s.set_core_override(g, Some("nestopia")).unwrap();
        assert_eq!(s.core_override(g).unwrap().as_deref(), Some("nestopia"));
        s.set_core_override(g, None).unwrap();
        assert_eq!(s.core_override(g).unwrap(), None);
    }
}
