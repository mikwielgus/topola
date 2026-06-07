// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
static PUFFIN_SERVER: std::sync::OnceLock<puffin_http::Server> = std::sync::OnceLock::new();

#[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
pub fn enable() {
    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    PUFFIN_SERVER.get_or_init(|| {
        eprintln!("Run this to view profiler data: puffin_viewer {server_addr}");
        puffin_http::Server::new(&server_addr).expect("puffin_http server")
    });
    puffin::set_scopes_on(true);
}

#[cfg(any(target_arch = "wasm32", not(feature = "profiler")))]
pub fn enable() {}

#[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
pub fn begin_frame() {
    puffin::GlobalProfiler::lock().new_frame();
}

#[cfg(any(target_arch = "wasm32", not(feature = "profiler")))]
pub fn begin_frame() {}

#[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
pub fn profiler_window(ctx: &egui::Context, open: &mut bool) {
    if *open {
        *open = puffin_egui::profiler_window(ctx);
    }
}

#[cfg(any(target_arch = "wasm32", not(feature = "profiler")))]
pub fn profiler_window(_ctx: &egui::Context, _open: &mut bool) {}

#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
        puffin::profile_scope!($name);
    };
}

#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiler"))]
        puffin::profile_function!();
    };
}
