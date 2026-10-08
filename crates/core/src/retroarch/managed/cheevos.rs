//! RetroAchievements in the managed RetroArch: login (user + token, never the password)
//! and hardcore mode. The token comes from `dorequest.php?r=login2`.

/// Achievement settings rombro writes into the config.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cheevos {
    /// (user name, login token); `None` = logged out.
    pub login: Option<(String, String)>,
    /// Hardcore: no savestates, rewind or cheats; unlocks count double on the site.
    pub hardcore: bool,
}

impl Cheevos {
    /// `retroarch.cfg` keys; logged out disables achievements and clears the credentials.
    pub fn keys(&self) -> Vec<(&'static str, String)> {
        let (enable, user, token) = match &self.login {
            Some((u, t)) => (true, u.as_str(), t.as_str()),
            None => (false, "", ""),
        };
        vec![
            ("cheevos_enable", enable.to_string()),
            ("cheevos_username", user.into()),
            ("cheevos_token", token.into()),
            ("cheevos_password", String::new()),
            ("cheevos_hardcore_mode_enable", self.hardcore.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_writes_token_never_password() {
        let c = Cheevos {
            login: Some(("kc".into(), "tok".into())),
            hardcore: true,
        };
        let keys = c.keys();
        assert!(keys.contains(&("cheevos_enable", "true".into())));
        assert!(keys.contains(&("cheevos_token", "tok".into())));
        assert!(keys.contains(&("cheevos_password", String::new())));
        assert!(keys.contains(&("cheevos_hardcore_mode_enable", "true".into())));
        let off = Cheevos::default().keys();
        assert!(off.contains(&("cheevos_enable", "false".into())));
        assert!(off.contains(&("cheevos_token", String::new())));
    }
}
