# libubox-sys / libubox

Rust bindings to OpenWrt's [libubox](https://git.openwrt.org/project/libubox.git):

- **`libubox-sys`** — raw `bindgen` FFI (links `libubox.so` / `libubox.a`).
- **`libubox`** — minimal RAII wrapper over the sys crate (currently only an
  `Uloop` guard and an owned `BlobBuf`; everything else still goes through
  `libubox_sys::*`).

The repo vendors libubox as a git submodule at `libubox-sys/vendor/libubox`,
pinned to a specific upstream commit, so consumers do not need an OpenWrt SDK
and do not have to install libubox system-wide.

## Build-host requirements

- A C compiler and CMake (>= 3.13).
- `pkg-config` and **`libjson-c-dev`** (or your distro's equivalent). libubox's
  CMake unconditionally `PKG_SEARCH_MODULE`s json-c, even for the core
  `libubox` library, so this is required even when the `json` cargo feature
  is *off*.
- `git submodule update --init` after cloning so `vendor/libubox` is populated.

## Cargo features (on `libubox-sys`)

| Feature   | Default | Effect |
|-----------|---------|--------|
| `json`    | off     | Also link `libblobmsg_json` and `libjson_script`, probe json-c via pkg-config, expose JSON-c-dependent FFI items. |
| `bindgen` | off     | Regenerate bindings at build time instead of using the committed pregenerated file. |
| `static`  | off     | Link libubox statically. Equivalent: set `LIBUBOX_STATIC=1`. Note: `libjson_script` is always linked dynamically — upstream provides no static target for it. |

## Environment variable overrides

For OpenWrt SDK / cross builds where libubox is already installed:

- `LIBUBOX_DIR` — prefix containing `include/libubox/*.h` and `lib/libubox.*`.
- `LIBUBOX_INCLUDE_DIR` — overrides include dir (parent of `libubox/`).
- `LIBUBOX_LIB_DIR` — overrides lib dir.
- `LIBUBOX_STATIC=1` — equivalent to the `static` feature.

If any of these are set, the vendored CMake build is skipped.

## Quick start

```toml
[dependencies]
libubox     = "0.1"
libubox-sys = "0.1"   # only if you need raw FFI directly
```

```rust
use libubox::{BlobBuf, Uloop};

let mut uloop = Uloop::new().expect("uloop already initialized");
let mut buf = BlobBuf::new().expect("blob_buf_init");
// ... register callbacks via libubox_sys::* against buf.as_mut_ptr(), then:
uloop.run();
```

## License

Dual licensed under MIT or Apache-2.0, at your option. The vendored libubox
sources are ISC; see [LICENSE-ISC](LICENSE-ISC).
