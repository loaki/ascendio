//! The one place that knows where the save data actually lives. Everything
//! else just hands this a JSON string and gets one back -- `quad_storage`
//! already abstracts browser localStorage (wasm) vs. a local file (native)
//! behind one API, so there is nothing platform-specific left to do here.

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
