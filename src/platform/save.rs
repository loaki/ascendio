//! Save storage: localStorage on wasm, a local file natively; never panics.

const KEY: &str = "ascendio_save_v1";
const SETTINGS_KEY: &str = "ascendio_settings_v1";
const CLOCK_KEY: &str = "ascendio_clock_v1";
/// Values that failed to load are kept as `<key>_bad_<n>`, this many at most.
const MAX_BAD: usize = 5;

pub fn write(json: &str) {
    store::set(KEY, json);
}

pub fn read() -> Option<String> {
    store::get(KEY)
}

pub fn clear() {
    store::remove(KEY);
}

pub fn write_settings(json: &str) {
    store::set(SETTINGS_KEY, json);
}

pub fn read_settings() -> Option<String> {
    store::get(SETTINGS_KEY)
}

pub fn write_clock(json: &str) {
    store::set(CLOCK_KEY, json);
}

pub fn read_clock() -> Option<String> {
    store::get(CLOCK_KEY)
}

pub fn keep_bad_save() {
    if let Some(raw) = read() {
        keep_bad(KEY, &raw);
    }
}

pub fn keep_bad_settings(raw: &str) {
    keep_bad(SETTINGS_KEY, raw);
}

/// `raw` under the first free `<key>_bad_<n>`; nothing if it is already
/// kept, or if all the slots are taken (the oldest are the ones worth having).
fn keep_bad(key: &str, raw: &str) {
    if raw.trim().is_empty() {
        return;
    }
    for n in 0..MAX_BAD {
        let slot = format!("{key}_bad_{n}");
        match store::get(&slot) {
            Some(kept) if kept == raw => return,
            Some(_) => {}
            None => {
                store::set(&slot, raw);
                return;
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod store {

    use std::sync::MutexGuard;

    use quad_storage::LocalStorage;

    fn storage() -> MutexGuard<'static, LocalStorage> {
        quad_storage::STORAGE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(key: &str) -> Option<String> {
        storage().get(key)
    }

    pub fn set(key: &str, value: &str) {
        let mut s = storage();
        if s.get(key).as_deref() != Some(value) {
            s.set(key, value);
        }
    }

    pub fn remove(key: &str) {
        storage().remove(key);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    //! Every key in one JSON file, rewritten whole on each change: to a
    //! temporary file first, synced, then renamed over the old one, so an
    //! interrupted write leaves the previous save intact.
    //!
    //! The file is the one `quad_storage` used to write (same name, same
    //! `{"local": {key: value}}` layout), so existing saves carry over.

    use std::collections::BTreeMap;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use serde::{Deserialize, Serialize};

    const FILE: &str = "local.data";

    #[derive(Default, Serialize, Deserialize)]
    struct Data {
        #[serde(default)]
        local: BTreeMap<String, String>,
    }

    struct Store {
        path: PathBuf,
        data: Data,
        /// False when the file on disk couldn't be read nor moved aside:
        /// it may still hold the save, so it is never written over.
        writable: bool,
    }

    fn store() -> MutexGuard<'static, Store> {
        static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
        STORE
            .get_or_init(|| Mutex::new(Store::open(super::dir::get().join(FILE))))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn get(key: &str) -> Option<String> {
        store().data.local.get(key).cloned()
    }

    pub fn set(key: &str, value: &str) {
        let mut s = store();
        if s.data.local.get(key).map(String::as_str) != Some(value) {
            s.data.local.insert(key.to_string(), value.to_string());
            s.flush();
        }
    }

    pub fn remove(key: &str) {
        let mut s = store();
        if s.data.local.remove(key).is_some() {
            s.flush();
        }
    }

    impl Store {
        /// A missing file is an empty store; a broken one too, once moved
        /// aside to `local.data.bad*`. One that can't be read at all (no
        /// permission, an I/O error) may be fine: the session starts empty
        /// and never writes over it.
        fn open(path: PathBuf) -> Self {
            let data = match std::fs::read_to_string(&path) {
                Ok(text) => serde_json::from_str(&text).ok(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(Data::default()),
                // Not UTF-8: broken, like a file that doesn't parse.
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => None,
                Err(e) => {
                    eprintln!("save: can't read {}: {e}; not saving", path.display());
                    return Self {
                        path,
                        data: Data::default(),
                        writable: false,
                    };
                }
            };
            let mut writable = true;
            let data = data.unwrap_or_else(|| {
                eprintln!("save: {} is unreadable, starting empty", path.display());
                writable = move_aside(&path);
                Data::default()
            });
            Self {
                path,
                data,
                writable,
            }
        }

        fn flush(&self) {
            if !self.writable {
                return;
            }
            if let Err(e) = self.try_flush() {
                eprintln!("save: can't write {}: {e}", self.path.display());
            }
        }

        fn try_flush(&self) -> std::io::Result<()> {
            let json = serde_json::to_string(&self.data)?;
            let tmp = self.path.with_extension("data.tmp");
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&tmp, &self.path)?;
            // The rename itself survives a power cut once the folder is synced.
            #[cfg(unix)]
            if let Some(dir) = self.path.parent() {
                let dir = if dir.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    dir
                };
                let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
            }
            Ok(())
        }
    }

    fn move_aside(path: &Path) -> bool {
        let bad = (0..super::MAX_BAD)
            .map(|n| path.with_extension(format!("data.bad{n}")))
            .find(|bad| !bad.exists())
            .unwrap_or_else(|| path.with_extension(format!("data.bad{}", super::MAX_BAD - 1)));
        std::fs::rename(path, bad).is_ok()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn scratch(name: &str) -> PathBuf {
            let dir =
                std::env::temp_dir().join(format!("ascendio-save-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir.join(FILE)
        }

        #[test]
        fn a_quad_storage_file_loads() {
            let path = scratch("legacy");
            // As nanoserde wrote it for quad-storage 0.1.3.
            std::fs::write(&path, r#"{"local":{"a":"{\"x\":\"1\\n\"}","b":"2"}}"#).unwrap();
            let s = Store::open(path);
            assert_eq!(s.data.local["a"], "{\"x\":\"1\\n\"}");
            assert_eq!(s.data.local["b"], "2");
        }

        #[test]
        fn a_broken_file_loads_empty_and_is_kept() {
            let path = scratch("broken");
            std::fs::write(&path, r#"{"local":{"a":"tru"#).unwrap();
            let s = Store::open(path.clone());
            assert!(s.data.local.is_empty());
            assert!(!path.exists());
            assert!(path.with_extension("data.bad0").exists());
        }

        #[test]
        fn a_write_round_trips_and_leaves_no_temporary() {
            let path = scratch("write");
            let mut s = Store::open(path.clone());
            s.data.local.insert("k".into(), "v\"\n".into());
            s.flush();
            assert!(!path.with_extension("data.tmp").exists());
            assert_eq!(Store::open(path).data.local["k"], "v\"\n");
        }
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod dir {
    pub fn get() -> std::path::PathBuf {
        std::path::PathBuf::new()
    }
}

#[cfg(target_os = "android")]
mod dir {
    //! The app's private files dir, asked of the activity: it differs per
    //! user, work profile and Private Space.

    use std::path::PathBuf;

    use macroquad::miniquad::call_object_method;
    use macroquad::miniquad::native::android::ACTIVITY;

    use crate::platform::notify::{clear, rust_string, with_jni};

    const FALLBACK: &str = "/data/data/com.loaki.ascendio/files";

    pub fn get() -> PathBuf {
        let dir = files_dir()
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| FALLBACK.to_string());
        let _ = std::fs::create_dir_all(&dir);
        PathBuf::from(dir)
    }

    fn files_dir() -> Option<String> {
        unsafe {
            with_jni(|env| {
                let file = call_object_method!(env, ACTIVITY, "getFilesDir", "()Ljava/io/File;");
                clear(env);
                if file.is_null() {
                    return None;
                }
                let path =
                    call_object_method!(env, file, "getAbsolutePath", "()Ljava/lang/String;");
                clear(env);
                rust_string(env, path)
            })
        }
    }
}
