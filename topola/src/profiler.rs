// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
