//! The game's clock: wall-clock seconds the player can't wind forward.
//!
//! Waits run while the app is closed, so they're measured against the time
//! of day, which the player controls. On Android the game anchors that time
//! to the uptime clock (`SystemClock.elapsedRealtime`), which a change of
//! the phone's time doesn't move: between two restarts, time only advances
//! as the uptime does. After a restart it trusts the phone's clock once and
//! anchors again. Everywhere, time never goes backwards.

use serde::{Deserialize, Serialize};

/// Where the trusted time stands against the device's uptime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    /// Trusted seconds at `uptime` (0: never anchored).
    wall: f64,
    /// The uptime, in seconds, when anchored.
    uptime: f64,
    /// Android's boot counter when anchored: another means a restart.
    boot: i64,
    /// The latest time handed out: never handed out earlier again.
    last: f64,
}

impl Anchor {
    /// Trusted seconds now. `wall` is the device's clock; `device` its
    /// uptime and boot count, where it has them (Android).
    pub fn now(&mut self, wall: f64, device: Option<(f64, i64)>) -> f64 {
        let t = match device {
            // Same boot: the uptime decides, whatever the clock says.
            Some((uptime, boot))
                if self.wall > 0.0 && boot == self.boot && uptime >= self.uptime =>
            {
                self.wall + (uptime - self.uptime)
            }
            // First launch, or the phone restarted: trust the clock once.
            Some((uptime, boot)) => {
                let t = wall.max(self.last);
                *self = Anchor {
                    wall: t,
                    uptime,
                    boot,
                    last: self.last,
                };
                t
            }
            None => wall,
        };
        self.last = self.last.max(t);
        self.last
    }
}

/// The trusted clock, kept with the settings so it survives starting over.
pub struct Clock {
    anchor: Anchor,
    /// Off for dev runs on a scratch save.
    persist: bool,
    saved: Anchor,
}

impl Clock {
    pub fn load(persist: bool) -> Self {
        let anchor = persist
            .then(crate::save::read_clock)
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Self {
            anchor,
            persist,
            saved: anchor,
        }
    }

    /// Trusted seconds now; `time_scale` is the dev speed-up of the device
    /// clock (desktop only).
    pub fn now(&mut self, time_scale: f64) -> f64 {
        let wall = macroquad::miniquad::date::now() * time_scale;
        let t = self.anchor.now(wall, device::uptime_and_boot());
        // A new anchor is written at once; the rest rides on the autosave.
        if self.anchor.wall != self.saved.wall {
            self.save();
        }
        t
    }

    pub fn save(&mut self) {
        if self.persist && self.anchor != self.saved {
            crate::save::write_clock(&serde_json::to_string(&self.anchor).unwrap_or_default());
            self.saved = self.anchor;
        }
    }
}

#[cfg(target_os = "android")]
mod device {
    //! Through the activity, like `notify.rs` (`android/main_activity_inject.java`).

    use macroquad::miniquad::call_method;
    use macroquad::miniquad::native::android::{attach_jni_env, ndk_sys, ACTIVITY};

    unsafe fn clear(env: *mut ndk_sys::JNIEnv) {
        if (**env).ExceptionCheck.unwrap()(env) != 0 {
            (**env).ExceptionClear.unwrap()(env);
        }
    }

    /// Seconds since the phone started, and how many times it has started.
    pub fn uptime_and_boot() -> Option<(f64, i64)> {
        static BOOT: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
        unsafe {
            let env = attach_jni_env();
            let ms = call_method!(CallLongMethod, env, ACTIVITY, "uptimeMs", "()J");
            clear(env);
            // It can't change while the game runs: asked once.
            let boot = *BOOT.get_or_init(|| {
                let b = call_method!(CallIntMethod, env, ACTIVITY, "bootCount", "()I");
                clear(env);
                i64::from(b)
            });
            (boot >= 0).then_some((ms as f64 / 1000.0, boot))
        }
    }
}

#[cfg(not(target_os = "android"))]
mod device {
    /// No uptime to trust: the device clock alone, never going backwards.
    pub fn uptime_and_boot() -> Option<(f64, i64)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winding_the_clock_forward_changes_nothing_until_a_restart() {
        let mut a = Anchor::default();
        assert_eq!(
            a.now(1000.0, Some((50.0, 7))),
            1000.0,
            "first launch: the clock"
        );
        // An hour of uptime later, the clock says a day has passed.
        assert_eq!(a.now(1000.0 + 86_400.0, Some((3650.0, 7))), 4600.0);
    }

    #[test]
    fn a_restart_trusts_the_clock_once_and_anchors_again() {
        let mut a = Anchor::default();
        a.now(1000.0, Some((50.0, 7)));
        assert_eq!(a.now(5000.0, Some((10.0, 8))), 5000.0, "restarted");
        assert_eq!(
            a.now(9999.0, Some((20.0, 8))),
            5010.0,
            "then the uptime again"
        );
    }

    #[test]
    fn time_never_goes_backwards() {
        let mut a = Anchor::default();
        assert_eq!(a.now(5000.0, None), 5000.0);
        assert_eq!(a.now(4000.0, None), 5000.0, "the clock wound back");
        assert_eq!(a.now(5100.0, None), 5100.0);
        // Nor across a restart with the clock wound back.
        let mut b = Anchor::default();
        b.now(5000.0, Some((100.0, 1)));
        assert_eq!(b.now(10.0, Some((5.0, 2))), 5000.0);
    }

    #[test]
    fn the_anchor_survives_a_save() {
        let mut a = Anchor::default();
        a.now(1000.0, Some((50.0, 7)));
        let mut back: Anchor = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(back.now(1e9, Some((60.0, 7))), 1010.0);
    }
}
