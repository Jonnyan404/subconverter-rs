use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/kv_bindings.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn kv_get(key: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_get_text(key: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_set(key: &str, value: &[u8]) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_set_text(key: &str, value: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_exists(key: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_del(key: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn kv_list(prefix: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn fetch_url(url: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn response_status(response: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn response_bytes(response: &JsValue) -> Result<Uint8Array, JsValue>;

    #[wasm_bindgen]
    pub fn getenv(name: &str, default_value: &str) -> String;

    #[wasm_bindgen(catch)]
    pub fn dummy() -> Result<JsValue, JsValue>;
}

// Helper to convert JsValue error to VfsError
use crate::vfs::VfsError;

pub fn js_error_to_vfs(err: JsValue, context: &str) -> VfsError {
    let msg = format!("{}: {:?}", context, err);
    log::error!("{}", msg);
    if context.contains("KV") {
        VfsError::StorageError(msg)
    } else if context.contains("Fetch") || context.contains("GitHub") {
        // Match GitHub context too
        VfsError::NetworkError(msg)
    } else {
        VfsError::Other(msg)
    }
}
