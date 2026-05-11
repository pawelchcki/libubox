# Agent notes

Cargo workspace: `libubox-sys` (raw FFI, vendors `vendor/libubox` submodule, builds via `cmake`) and `libubox` (safe wrapper).

Build needs `cmake` + `c++`; on hosts without them run inside toolbox: `toolbox run --container fedora-toolbox-44 bash -lc "cargo test --workspace"`. The `json` feature additionally needs json-c dev headers.

System mode skips the vendored cmake build: set `LIBUBOX_INCLUDE_DIR` + `LIBUBOX_LIB_DIR` (or `LIBUBOX_DIR=<prefix>`).

Releases: `cargo release -p <crate> patch --execute` (config in `release.toml`) bumps + commits + tags + pushes. The tag (`<crate>-v<version>`) triggers `.github/workflows/release.yml`, which uses crates.io Trusted Publishing — no static token — and creates a GitHub release. Do not change the tag scheme without updating that workflow.
