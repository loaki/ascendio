//! Save storage: localStorage on wasm, a local file natively (`quad_storage`).

const KEY: &str = "ascendio_save_v1";

pub fn write(json: &str) {
    quad_storage::STORAGE.lock().unwrap().set(KEY, json);
}

pub fn read() -> Option<String> {
    quad_storage::STORAGE.lock().unwrap().get(KEY)
}

pub fn clear() {
    quad_storage::STORAGE.lock().unwrap().remove(KEY);
}
