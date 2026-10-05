//! Detection of copier/emulator headers that RetroArch databases hash without.

/// Number of leading bytes needed by [`detect`].
pub const PROBE_LEN: usize = 128;

/// Known header kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Header {
    Ines,
    Fds,
    Atari7800,
    Lynx,
    SnesCopier,
}

impl Header {
    pub fn size(self) -> u64 {
        match self {
            Self::Ines | Self::Fds => 16,
            Self::Lynx => 64,
            Self::Atari7800 => 128,
            Self::SnesCopier => 512,
        }
    }
}

/// Detects a header from the first bytes of a file, its total size and lowercase extension.
pub fn detect(probe: &[u8], size: u64, ext: &str) -> Option<Header> {
    let h = if probe.starts_with(b"NES\x1a") {
        Header::Ines
    } else if probe.starts_with(b"FDS\x1a") {
        Header::Fds
    } else if probe.starts_with(b"LYNX") {
        Header::Lynx
    } else if probe.get(1..10) == Some(b"ATARI7800") {
        Header::Atari7800
    } else if matches!(ext, "smc" | "sfc" | "swc" | "fig") && size % 1024 == 512 {
        Header::SnesCopier
    } else {
        return None;
    };
    (size > h.size()).then_some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_magic_headers() {
        assert_eq!(detect(b"NES\x1a\x02", 40976, "nes"), Some(Header::Ines));
        assert_eq!(detect(b"FDS\x1a", 65516, "fds"), Some(Header::Fds));
        assert_eq!(detect(b"LYNX\0", 1000, "lnx"), Some(Header::Lynx));
        assert_eq!(
            detect(b"\x01ATARI7800", 1000, "a78"),
            Some(Header::Atari7800)
        );
        assert_eq!(detect(b"NES\x1a", 16, "nes"), None);
        assert_eq!(detect(b"\0\0\0\0", 40960, "nes"), None);
    }

    #[test]
    fn snes_copier_by_size() {
        assert_eq!(
            detect(b"", 512 + 1024 * 1024, "smc"),
            Some(Header::SnesCopier)
        );
        assert_eq!(detect(b"", 1024 * 1024, "sfc"), None);
        assert_eq!(detect(b"", 512 + 1024 * 1024, "bin"), None);
    }
}
