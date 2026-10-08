//! How the managed RetroArch shows games: fullscreen or in a window of a given scale.
//! Stored as setting `ra_display` (`fullscreen`, `window:<scale>`); unset leaves the
//! choice to RetroArch's own menu.

use std::fmt;
use std::str::FromStr;

/// Default window scale (×3: 960×720 for 320×240 games).
pub const DEFAULT_SCALE: u8 = 3;
pub const SCALES: std::ops::RangeInclusive<u8> = 1..=6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Fullscreen,
    /// Windowed, sized to `scale` × the game's resolution; position and size are remembered.
    Window {
        scale: u8,
    },
}

impl Display {
    /// `retroarch.cfg` keys this mode sets.
    pub fn keys(self) -> Vec<(&'static str, String)> {
        let b = |v: bool| v.to_string();
        match self {
            Display::Fullscreen => vec![
                ("video_fullscreen", b(true)),
                ("video_windowed_fullscreen", b(true)),
            ],
            Display::Window { scale } => vec![
                ("video_fullscreen", b(false)),
                ("video_scale", scale.to_string()),
                ("video_window_custom_size_enable", b(false)),
                ("video_window_save_positions", b(false)),
            ],
        }
    }
}

impl fmt::Display for Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Display::Fullscreen => f.write_str("fullscreen"),
            Display::Window { scale } => write!(f, "window:{scale}"),
        }
    }
}

impl FromStr for Display {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || format!("{s}: expected fullscreen, window or window:<1-6>");
        match s.split_once(':') {
            None if s == "fullscreen" => Ok(Display::Fullscreen),
            None if s == "window" => Ok(Display::Window {
                scale: DEFAULT_SCALE,
            }),
            Some(("window", n)) => n
                .parse()
                .ok()
                .filter(|n| SCALES.contains(n))
                .map(|scale| Display::Window { scale })
                .ok_or_else(bad),
            _ => Err(bad()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prints() {
        for s in ["fullscreen", "window:2", "window:6"] {
            assert_eq!(s.parse::<Display>().unwrap().to_string(), s);
        }
        assert_eq!("window".parse(), Ok(Display::Window { scale: 3 }));
        assert!("window:9".parse::<Display>().is_err());
        assert!("big".parse::<Display>().is_err());
    }

    #[test]
    fn window_turns_fullscreen_off() {
        let keys = Display::Window { scale: 4 }.keys();
        assert!(keys.contains(&("video_fullscreen", "false".into())));
        assert!(keys.contains(&("video_scale", "4".into())));
    }
}
