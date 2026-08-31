//! A Dioxus-native documentation framework.
//!
//! A generated site only needs [`types`], so it depends on this crate with
//! `default-features = false` and pulls in nothing else. The parsing and codegen behind
//! `dxpress` sit behind the `core` and `build` features.

pub mod types;

#[cfg(feature = "core")]
pub mod core;

#[cfg(feature = "build")]
pub mod build;

#[cfg(all(target_arch = "wasm32", feature = "core"))]
compile_error!(
    "dioxus-press pulls its parser and highlighter into a wasm build. A site needs only \
     the shared types: depend on it with `default-features = false`."
);
