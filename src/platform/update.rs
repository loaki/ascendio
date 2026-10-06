//! Android launch check for a newer GitHub release; never blocks the game.

use crate::platform::net::{self, Request};

const LATEST: &str = "https://api.github.com/repos/loaki/ascendio/releases/latest";

pub struct Update {
    check: Option<Request>,
    pub page: Option<String>,
}

impl Update {
    pub fn new() -> Self {
        let stamped = option_env!("ASCENDIO_VERSION").is_some_and(|v| v.starts_with('v'));
        Self {
            check: (cfg!(target_os = "android") && stamped)
                .then(|| net::request("GET", LATEST, None)),
            page: None,
        }
    }

    pub fn update(&mut self) {
        let Some(req) = &mut self.check else { return };
        let Some(reply) = req.poll() else { return };
        self.check = None;
        if let Ok(text) = reply {
            self.page = newer_page(&text, option_env!("ASCENDIO_VERSION").unwrap_or(""));
        }
    }

    pub fn open(&self) {
        if let Some(page) = &self.page {
            open_url(page);
        }
    }
}

/// The release page in a GitHub "latest release" reply, if its tag is newer
/// than `current`. Tags are `vYYYY.MM.DD-HHMM`, so they sort as text.
fn newer_page(reply: &str, current: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(reply).ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    let page = json.get("html_url")?.as_str()?;
    (tag.starts_with('v') && tag > current).then(|| page.to_string())
}

#[cfg(target_os = "android")]
fn open_url(url: &str) {
    use macroquad::miniquad::call_void_method;
    use macroquad::miniquad::native::android::ACTIVITY;

    use crate::platform::notify::{clear, with_jni};
    unsafe {
        with_jni(|env| {
            let c = std::ffi::CString::new(url.replace('\0', "")).unwrap_or_default();
            let s = (**env).NewStringUTF.unwrap()(env, c.as_ptr());
            call_void_method!(env, ACTIVITY, "openUrl", "(Ljava/lang/String;)V", s);
            clear(env);
        })
    }
}

#[cfg(not(target_os = "android"))]
fn open_url(_url: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLY: &str = r#"{"tag_name":"v2026.10.05-1200","html_url":"https://github.com/loaki/ascendio/releases/tag/v2026.10.05-1200"}"#;

    #[test]
    fn newer_tag_gives_the_page() {
        assert!(newer_page(REPLY, "v2026.10.04-0900").is_some());
    }

    #[test]
    fn same_older_or_broken_gives_nothing() {
        assert!(newer_page(REPLY, "v2026.10.05-1200").is_none());
        assert!(newer_page(REPLY, "v2026.11.01-0000").is_none());
        assert!(newer_page("not json", "v1").is_none());
        assert!(newer_page(r#"{"message":"Not Found"}"#, "v1").is_none());
    }
}
