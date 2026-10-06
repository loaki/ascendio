//! The "genome ready" notification, scheduled via JobScheduler on Android only.

use crate::game::{Game, Phase};

/// Keeps the pending notification in step with the game: one for the
/// running wait while the setting is on, none otherwise. A genome that is
/// already waiting keeps its notification until it is opened.
/// `time_scale` is the dev clock speed-up, so the job fires in real time.
pub fn sync(game: &Game, on: bool, now: f64, time_scale: f64) {
    match game.cycle {
        Some(c) if on => schedule(
            c.remaining(now) / time_scale,
            &format!("{} Ma have passed on your planet. Tap to express it.", c.ma),
        ),
        _ if on && game.phase() == Phase::Genome => {}
        _ => cancel(),
    }
}

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(not(target_os = "android"))]
pub use elsewhere::*;

#[cfg(target_os = "android")]
mod android {

    use macroquad::miniquad::native::android::{attach_jni_env, ndk_sys, ACTIVITY};
    use macroquad::miniquad::{call_bool_method, call_void_method};

    pub unsafe fn clear(env: *mut ndk_sys::JNIEnv) {
        if (**env).ExceptionCheck.unwrap()(env) != 0 {
            (**env).ExceptionClear.unwrap()(env);
        }
    }

    /// Runs JNI calls in a local reference frame of their own. The game
    /// thread never returns to Java, so a local reference is only freed by
    /// hand, and miniquad's `call_method!` leaks the class it looks up:
    /// without a frame they pile up until the VM aborts (512 on Android 7).
    /// Nothing `f` gets from Java outlives it.
    pub unsafe fn with_jni<R>(f: impl FnOnce(*mut ndk_sys::JNIEnv) -> R) -> R {
        let env = attach_jni_env();
        let framed = (**env).PushLocalFrame.unwrap()(env, 16) == 0;
        clear(env);
        let out = f(env);
        clear(env);
        if framed {
            (**env).PopLocalFrame.unwrap()(env, std::ptr::null_mut());
        }
        out
    }

    pub unsafe fn rust_string(env: *mut ndk_sys::JNIEnv, s: ndk_sys::jobject) -> Option<String> {
        if s.is_null() {
            return None;
        }
        let chars = (**env).GetStringUTFChars.unwrap()(env, s, std::ptr::null_mut());
        if chars.is_null() {
            return None;
        }
        let out = std::ffi::CStr::from_ptr(chars)
            .to_string_lossy()
            .into_owned();
        (**env).ReleaseStringUTFChars.unwrap()(env, s, chars);
        Some(out)
    }

    unsafe fn call(method: &str) {
        with_jni(|env| {
            call_void_method!(env, ACTIVITY, method, "()V");
        });
    }

    pub fn supported() -> bool {
        true
    }

    pub fn allowed() -> bool {
        unsafe {
            with_jni(|env| call_bool_method!(env, ACTIVITY, "notificationsAllowed", "()Z") != 0)
        }
    }

    pub fn ask() {
        unsafe { call("askNotifications") }
    }

    pub fn open_settings() {
        unsafe { call("openNotificationSettings") }
    }

    pub fn schedule(delay_secs: f64, body: &str) {
        let Ok(body) = std::ffi::CString::new(body) else {
            return;
        };
        unsafe {
            with_jni(|env| {
                let jbody = (**env).NewStringUTF.unwrap()(env, body.as_ptr());
                let ms = (delay_secs.max(0.0) * 1000.0) as ndk_sys::jlong;
                call_void_method!(
                    env,
                    ACTIVITY,
                    "scheduleGenome",
                    "(JLjava/lang/String;)V",
                    ms,
                    jbody
                );
            });
        }
    }

    pub fn cancel() {
        unsafe { call("cancelGenome") }
    }
}

#[cfg(not(target_os = "android"))]
mod elsewhere {
    pub fn supported() -> bool {
        false
    }

    pub fn allowed() -> bool {
        false
    }

    pub fn ask() {}

    pub fn open_settings() {}

    pub fn schedule(_delay_secs: f64, _body: &str) {}

    pub fn cancel() {}
}
