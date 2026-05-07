//! Raw FFI bindings to OpenWrt's [libubox].
//!
//! See the workspace `README.md` for build-host requirements (CMake,
//! `pkg-config`, `libjson-c-dev`) and feature flags (`json`, `bindgen`,
//! `static`).
//!
//! The committed bindings live in `src/bindings/pregenerated.rs` and are
//! used unless the `bindgen` feature is enabled.
//!
//! [libubox]: https://git.openwrt.org/project/libubox.git

mod bindings {
    #![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
    #![allow(deref_nullptr, clippy::all)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
