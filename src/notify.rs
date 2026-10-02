//! The "genome ready" notification.
//!
//! The game is usually closed when a wait ends, so it can't post anything
//! itself. On Android it hands the end of the wait to JobScheduler
//! (`java/com/loaki/ascendio/GenomeJob.java`), which posts the notification
//! on time. Nothing else supports it: the web build can't wake up while its
//! tab sleeps.

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
    //! Calls into the methods `android/main_activity_inject.java` adds to
    //! miniquad's MainActivity. App classes can't be looked up from the game
    //! thread (`FindClass` only sees the system's), so everything goes
    //! through the activity object.

    use macroquad::miniquad::native::android::{attach_jni_env, ndk_sys, ACTIVITY};
    use macroquad::miniquad::{call_bool_method, call_void_method};

    /// Drops a Java exception rather than letting the next JNI call abort.
    unsafe fn clear(env: *mut ndk_sys::JNIEnv) {
        if (**env).ExceptionCheck.unwrap()(env) != 0 {
            (**env).ExceptionClear.unwrap()(env);
        }
    }

    unsafe fn call(method: &str) {
        let env = attach_jni_env();
        call_void_method!(env, ACTIVITY, method, "()V");
        clear(env);
    }

    pub fn supported() -> bool {
        true
    }

    /// Whether Android lets the app post notifications at all.
    pub fn allowed() -> bool {
        unsafe {
            let env = attach_jni_env();
            let ok = call_bool_method!(env, ACTIVITY, "notificationsAllowed", "()Z");
            clear(env);
            ok != 0
        }
    }

    /// Android 13+: the system's "allow notifications?" prompt.
    pub fn ask() {
        unsafe { call("askNotifications") }
    }

    /// The app's notification settings, for when the prompt was refused.
    pub fn open_settings() {
        unsafe { call("openNotificationSettings") }
    }

    pub fn schedule(delay_secs: f64, body: &str) {
        let Ok(body) = std::ffi::CString::new(body) else {
            return;
        };
        unsafe {
            let env = attach_jni_env();
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
            clear(env);
            (**env).DeleteLocalRef.unwrap()(env, jbody);
        }
    }

    /// Unschedules the job and takes down a posted notification.
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
