//! Claim Lands application entrypoints for Android, iOS, and WebAssembly.
#[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
mod app;

/// Start the browser game in the page's game canvas.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    app::run(None).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}

/// NativeActivity entrypoint; the platform ABI requires a stable exported symbol.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    if let Err(error) = app::run(Some(android_app)) {
        eprintln!("{error}");
    }
}

/// Called by the minimal iOS application bootstrap on the main thread.
#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn claimlands_start() {
    if let Err(error) = app::run(None) {
        eprintln!("{error}");
    }
}
