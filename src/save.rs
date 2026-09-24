//! Save storage: localStorage on wasm, a local file natively (`quad_storage`).

use std::sync::MutexGuard;

use quad_storage::LocalStorage;

const KEY: &str = "ascendio_save_v1";

/// `quad_storage` keeps `local.data` in the working directory, which on
/// Android is `/` and read-only (the write panics). Move into the app's own
/// data dir first; the package name is `package_name` in Cargo.toml.
fn storage() -> MutexGuard<'static, LocalStorage> {
    #[cfg(target_os = "android")]
    {
        static CD: std::sync::Once = std::sync::Once::new();
        CD.call_once(|| {
            let dir = "/data/data/com.ascendio.game/files";
            let _ = std::fs::create_dir_all(dir);
            let _ = std::env::set_current_dir(dir);
        });
    }
    quad_storage::STORAGE.lock().unwrap()
}

pub fn write(json: &str) {
    storage().set(KEY, json);
}

pub fn read() -> Option<String> {
    storage().get(KEY)
}

pub fn clear() {
    storage().remove(KEY);
}
