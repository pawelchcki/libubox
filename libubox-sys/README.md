# libubox-sys

Raw `bindgen` FFI bindings to OpenWrt's [libubox](https://git.openwrt.org/project/libubox.git) (blob, blobmsg, uloop, avl, kvlist, vlist, ulog, usock, md5, runqueue, ustream, safe_list).

For an idiomatic Rust API, use the [`libubox`](https://crates.io/crates/libubox) crate instead.

## Usage

```toml
[dependencies]
libubox-sys = "0.0.1"
```

The crate vendors libubox as a git submodule and builds it via CMake by default, so consumers do not need an OpenWrt SDK or a system libubox install.

## Build prerequisites

- A C compiler and **CMake >= 3.13**
- `pkg-config` and **json-c dev headers** (`libjson-c-dev` / `json-c-devel`) — required even with the `json` feature off, because libubox's own CMakeLists unconditionally probes json-c.

## Cargo features

| Feature   | Default | Effect |
|-----------|---------|--------|
| `json`    | off     | Link `libblobmsg_json` + `libjson_script` and expose their FFI items. |
| `bindgen` | off     | Regenerate bindings at build time instead of using the committed pregenerated file. |
| `static`  | off     | Statically link `libubox` (equivalent to `LIBUBOX_STATIC=1`). `libjson_script` is always linked dynamically — upstream provides no static target. |

## System-mode build (skip the vendored CMake build)

Useful for OpenWrt SDK / cross builds where libubox is already installed:

- `LIBUBOX_DIR` — prefix with `include/libubox/*.h` and `lib/libubox.*`.
- `LIBUBOX_INCLUDE_DIR` / `LIBUBOX_LIB_DIR` — fine-grained overrides.
- `LIBUBOX_STATIC=1` — equivalent to the `static` feature.

If any of those are set, the vendored CMake build is skipped.

## Version & ABI

The crate version tracks the wrapper API, not libubox. The vendored libubox commit is pinned in `vendor/libubox` and recorded in `+libubox-<sha>` build-metadata where applicable.

MSRV: 1.90.

## License

Dual licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. The vendored libubox sources are under ISC.
