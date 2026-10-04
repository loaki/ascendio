//! Save storage: localStorage on wasm, a local file natively (`quad_storage`).

use std::sync::MutexGuard;

use quad_storage::LocalStorage;

const KEY: &str = "ascendio_save_v1";
/// Settings live apart from the game, so starting over keeps them.
const SETTINGS_KEY: &str = "ascendio_settings_v1";
/// The trusted clock's anchor (`clock.rs`), apart from the game too.
const CLOCK_KEY: &str = "ascendio_clock_v1";

/// `quad_storage` keeps `local.data` in the working directory, which on
/// Android is `/` and read-only (the write panics). Move into the app's own
/// data dir first; the package name is `package_name` in Cargo.toml.
fn storage() -> MutexGuard<'static, LocalStorage> {
    #[cfg(target_os = "android")]
    {
        static CD: std::sync::Once = std::sync::Once::new();
        CD.call_once(|| {
            let dir = "/data/data/com.loaki.ascendio/files";
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

pub fn write_settings(json: &str) {
    storage().set(SETTINGS_KEY, json);
}

pub fn read_settings() -> Option<String> {
    storage().get(SETTINGS_KEY)
}

pub fn write_clock(json: &str) {
    storage().set(CLOCK_KEY, json);
}

pub fn read_clock() -> Option<String> {
    storage().get(CLOCK_KEY)
}
