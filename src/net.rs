//! HTTP for the leaderboard, and a native text box for the player's name.
//!
//! Each platform has its own way out: `fetch` and `prompt` on the web
//! (`web/ascendio.js`), a Java thread and dialog on Android
//! (`android/main_activity_inject.java`), and `curl` on the desktop, which
//! only runs dev builds. Requests never block a frame: start one, then poll.

/// The outcome of a finished request.
pub type Reply = Result<String, Failure>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Failure {
    /// Offline, a server error, or told to slow down: worth trying again.
    Retry,
    /// The server refused this request (a 4xx): sending it again won't help.
    Rejected,
}

/// What an HTTP status means for the request (`None` for a success).
pub fn failure_for(code: u32) -> Option<Failure> {
    match code {
        200..=299 => None,
        400..=499 if code != 429 => Some(Failure::Rejected),
        _ => Some(Failure::Retry),
    }
}

pub use backend::{ask_text, request, Request};

#[cfg(target_arch = "wasm32")]
mod backend {
    use sapp_jsutils::{JsObject, JsObjectWeak};

    use super::{failure_for, Failure, Reply};

    extern "C" {
        fn ascendio_http(method: JsObjectWeak, url: JsObjectWeak, body: JsObjectWeak) -> u32;
        fn ascendio_http_status(id: u32) -> u32;
        fn ascendio_http_take(id: u32) -> JsObject;
        fn ascendio_prompt(title: JsObjectWeak, current: JsObjectWeak) -> u32;
        fn ascendio_prompt_text() -> JsObject;
    }

    /// Checked against the plugin's version by `gl.js`.
    #[no_mangle]
    pub extern "C" fn ascendio_net_crate_version() -> u32 {
        1
    }

    pub struct Request(u32);

    /// `body` makes it a JSON POST-style request.
    pub fn request(method: &str, url: &str, body: Option<&str>) -> Request {
        let (m, u, b) = (
            JsObject::string(method),
            JsObject::string(url),
            JsObject::string(body.unwrap_or("")),
        );
        Request(unsafe { ascendio_http(m.weak(), u.weak(), b.weak()) })
    }

    impl Request {
        /// `None` while it is in flight.
        pub fn poll(&mut self) -> Option<Reply> {
            // 0 in flight, 1 no answer at all, else the HTTP status.
            let status = unsafe { ascendio_http_status(self.0) };
            if status == 0 {
                return None;
            }
            let mut text = String::new();
            unsafe { ascendio_http_take(self.0) }.to_string(&mut text);
            Some(match status {
                1 => Err(Failure::Retry),
                code => failure_for(code).map_or(Ok(text), Err),
            })
        }
    }

    /// The browser's own prompt; it blocks, so the answer is immediate.
    /// `Some(None)` when cancelled.
    pub fn ask_text(title: &str, current: &str) -> Option<Option<String>> {
        let (t, c) = (JsObject::string(title), JsObject::string(current));
        if unsafe { ascendio_prompt(t.weak(), c.weak()) } == 0 {
            return Some(None);
        }
        let mut text = String::new();
        unsafe { ascendio_prompt_text() }.to_string(&mut text);
        Some(Some(text))
    }
}

#[cfg(target_os = "android")]
mod backend {
    //! Like `notify.rs`: everything goes through the activity object, as app
    //! classes can't be looked up from the game thread, and each call runs
    //! in a local reference frame (`notify::with_jni`).

    use macroquad::miniquad::native::android::{ndk_sys, ACTIVITY};
    use macroquad::miniquad::{call_int_method, call_object_method, call_void_method};

    use super::{failure_for, Failure, Reply};
    use crate::notify::{clear, rust_string, with_jni};

    unsafe fn jstring(env: *mut ndk_sys::JNIEnv, s: &str) -> ndk_sys::jstring {
        let c = std::ffi::CString::new(s.replace('\0', "")).unwrap_or_default();
        (**env).NewStringUTF.unwrap()(env, c.as_ptr())
    }

    pub struct Request(i32);

    pub fn request(method: &str, url: &str, body: Option<&str>) -> Request {
        unsafe {
            with_jni(|env| {
                let (m, u, b) = (
                    jstring(env, method),
                    jstring(env, url),
                    jstring(env, body.unwrap_or("")),
                );
                let id = call_int_method!(
                    env,
                    ACTIVITY,
                    "httpStart",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                    m,
                    u,
                    b
                );
                clear(env);
                Request(id)
            })
        }
    }

    impl Request {
        pub fn poll(&mut self) -> Option<Reply> {
            unsafe {
                with_jni(|env| {
                    // 0 in flight, 1 no answer at all, else the HTTP status.
                    let status = call_int_method!(env, ACTIVITY, "httpStatus", "(I)I", self.0);
                    clear(env);
                    if status == 0 {
                        return None;
                    }
                    let text = call_object_method!(
                        env,
                        ACTIVITY,
                        "httpTake",
                        "(I)Ljava/lang/String;",
                        self.0
                    );
                    clear(env);
                    let text = rust_string(env, text).unwrap_or_default();
                    Some(match status {
                        1 => Err(Failure::Retry),
                        code => failure_for(code as u32).map_or(Ok(text), Err),
                    })
                })
            }
        }
    }

    /// The dialog answers later: the first call opens it and returns
    /// `None`, later calls return the answer once (`Some(None)` if cancelled).
    pub fn ask_text(title: &str, current: &str) -> Option<Option<String>> {
        unsafe {
            with_jni(|env| {
                let status = call_int_method!(env, ACTIVITY, "promptStatus", "()I");
                clear(env);
                match status {
                    // Nothing asked yet: open the dialog.
                    0 => {
                        let (t, c) = (jstring(env, title), jstring(env, current));
                        call_void_method!(
                            env,
                            ACTIVITY,
                            "promptOpen",
                            "(Ljava/lang/String;Ljava/lang/String;)V",
                            t,
                            c
                        );
                        None
                    }
                    // Still open.
                    1 => None,
                    _ => {
                        let text = call_object_method!(
                            env,
                            ACTIVITY,
                            "promptTake",
                            "()Ljava/lang/String;"
                        );
                        clear(env);
                        Some(rust_string(env, text))
                    }
                }
            })
        }
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod backend {
    //! Dev builds: `curl` in a thread, so there is no HTTP stack to compile.

    use std::process::Command;
    use std::sync::mpsc::{self, Receiver, TryRecvError};

    use super::{failure_for, Failure, Reply};

    pub struct Request(Receiver<Reply>);

    pub fn request(method: &str, url: &str, body: Option<&str>) -> Request {
        let (tx, rx) = mpsc::channel();
        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-m", "10", "-w", "\n%{http_code}", "-X", method, url]);
        if let Some(body) = body {
            cmd.args([
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                body,
            ]);
        }
        std::thread::spawn(move || {
            let reply = cmd.output().ok().and_then(|out| {
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                let (body, code) = text.rsplit_once('\n')?;
                // curl prints 000 when nothing answered.
                let code: u32 = code.trim().parse().ok().filter(|&c| c != 0)?;
                Some(failure_for(code).map_or(Ok(body.to_string()), Err))
            });
            let _ = tx.send(reply.unwrap_or(Err(Failure::Retry)));
        });
        Request(rx)
    }

    impl Request {
        pub fn poll(&mut self) -> Option<Reply> {
            match self.0.try_recv() {
                Ok(reply) => Some(reply),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(Failure::Retry)),
            }
        }
    }

    /// No native text box: the game takes the keyboard instead.
    pub fn ask_text(_title: &str, _current: &str) -> Option<Option<String>> {
        None
    }
}

/// Whether `ask_text` shows a box of its own; if not, the game reads the
/// keyboard itself.
pub fn has_text_box() -> bool {
    cfg!(any(target_arch = "wasm32", target_os = "android"))
}
