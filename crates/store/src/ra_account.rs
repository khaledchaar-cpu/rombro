//! RetroAchievements account: login (password → token, the password is never stored),
//! hardcore choice, and the settings the managed RetroArch gets from them.

use crate::{Result, Store};
use rombro_core::retroarch::managed::{Managed, cheevos::Cheevos};
use std::io;

const LOGIN: &str = "https://retroachievements.org/dorequest.php";
const USER: &str = "ra.user";
const TOKEN: &str = "ra.token";
const HARDCORE: &str = "ra.hardcore";

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LoginReply {
    success: bool,
    user: Option<String>,
    token: Option<String>,
    error: Option<String>,
}

/// Logs in and returns (user name as RA spells it, token).
pub fn ra_login(user: &str, password: &str) -> io::Result<(String, String)> {
    let reply = ureq::post(LOGIN).header("User-Agent", "rombro").send_form([
        ("r", "login2"),
        ("u", user),
        ("p", password),
    ]);
    // RA answers a wrong password with 401 and the JSON error body.
    let body = match reply {
        Ok(mut r) => r.body_mut().read_to_string(),
        Err(ureq::Error::StatusCode(401)) => {
            return Err(io::Error::other("invalid user name or password"));
        }
        Err(e) => return Err(io::Error::other(e)),
    }
    .map_err(io::Error::other)?;
    parse_login(&body)
}

fn parse_login(body: &str) -> io::Result<(String, String)> {
    let r: LoginReply = serde_json::from_str(body).map_err(io::Error::other)?;
    match (r.success, r.user, r.token) {
        (true, Some(u), Some(t)) => Ok((u, t)),
        _ => Err(io::Error::other(
            r.error.unwrap_or_else(|| "login failed".into()),
        )),
    }
}

impl Store {
    /// Stored login (user, token).
    pub fn ra_account(&self) -> Result<Option<(String, String)>> {
        Ok(match (self.setting(USER)?, self.setting(TOKEN)?) {
            (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => Some((u, t)),
            _ => None,
        })
    }

    /// Stores (`Some`) or forgets the login.
    pub fn set_ra_account(&self, login: Option<(&str, &str)>) -> Result<()> {
        let (u, t) = login.unwrap_or(("", ""));
        self.set_setting(USER, u)?;
        self.set_setting(TOKEN, t)
    }

    pub fn ra_hardcore(&self) -> Result<bool> {
        Ok(self.setting(HARDCORE)?.as_deref() == Some("true"))
    }

    pub fn set_ra_hardcore(&self, on: bool) -> Result<()> {
        self.set_setting(HARDCORE, &on.to_string())
    }

    /// Achievement settings for the config; `None` until the user logged in or out once
    /// in rombro (a login made in RetroArch's own menu stays untouched).
    pub fn ra_cheevos(&self) -> Result<Option<Cheevos>> {
        if self.setting(USER)?.is_none() {
            return Ok(None);
        }
        Ok(Some(Cheevos {
            login: self.ra_account()?,
            hardcore: self.ra_hardcore()?,
        }))
    }

    /// `m` with the stored display mode and achievement settings.
    pub fn ra_prefs(&self, m: Managed) -> Result<Managed> {
        Ok(Managed {
            display: self.ra_display()?,
            cheevos: self.ra_cheevos()?,
            ..m
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_reply() {
        let ok = r#"{"Success":true,"User":"KC","Token":"abc","Score":5}"#;
        assert_eq!(parse_login(ok).unwrap(), ("KC".into(), "abc".into()));
        let bad =
            r#"{"Success":false,"Error":"Invalid User/Password combination. Please try again"}"#;
        assert!(
            parse_login(bad)
                .unwrap_err()
                .to_string()
                .contains("Invalid")
        );
    }

    #[test]
    fn account_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.ra_cheevos().unwrap(), None);
        s.set_ra_account(Some(("kc", "tok"))).unwrap();
        s.set_ra_hardcore(true).unwrap();
        let c = s.ra_cheevos().unwrap().unwrap();
        assert_eq!(c.login, Some(("kc".into(), "tok".into())));
        assert!(c.hardcore);
        s.set_ra_account(None).unwrap();
        assert_eq!(s.ra_cheevos().unwrap().unwrap().login, None);
    }
}
