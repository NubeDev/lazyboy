// Tauri's `generate_context!` macro reads codegen written by
// `tauri_build::build()` into OUT_DIR. Only the `app` feature build (the
// GUI shell) needs it; default builds compile just the command-body
// logic, so both the build dependency and this call are gated on the
// feature and stay inert there.
fn main() {
    #[cfg(feature = "app")]
    tauri_build::build();
}
