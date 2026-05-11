# libubox

Partial safe Rust wrapper around OpenWrt's [libubox](https://git.openwrt.org/project/libubox.git). Built on [`libubox-sys`](https://crates.io/crates/libubox-sys); fall through to that crate when you need FFI not yet wrapped here.

## Usage

```toml
[dependencies]
libubox = "0.0.1"
```

```rust
use libubox::{BlobmsgBuf, BlobmsgParser, BlobmsgType};

let mut buf = BlobmsgBuf::new()?;
buf.add_string(c"hostname", c"router")?;
buf.add_u32(c"uptime", 12_345)?;
buf.add_array(c"load", |arr| {
    arr.add_double(c"", 0.42)?;
    arr.add_double(c"", 0.17)?;
    Ok(())
})?;

let mut p = BlobmsgParser::new();
p.field(c"hostname", BlobmsgType::String)
 .field(c"uptime",   BlobmsgType::Int32);
let fields = p.parse(buf.root())?;

assert_eq!(fields[0].and_then(|a| a.as_str()), Some(c"router"));
assert_eq!(fields[1].and_then(|a| a.as_u32()), Some(12_345));
```

A runnable end-to-end example (cross-built and verified on an OpenWrt aarch64 router) lives in `examples/sysinfo.rs`:

```sh
cargo run --example sysinfo --features json
```

## Currently wrapped

- `BlobBuf` / `BlobAttr` / `BlobIter` — owned `blob_buf` (RAII), iteration, typed adders.
- `BlobmsgBuf` / `BlobmsgAttr` / `BlobmsgIter` — typed blobmsg construction, decoding, JSON formatting.
- `BlobmsgParser` — fixed-shape policy-driven parsing.
- `Uloop` — RAII guard for `uloop_init` / `uloop_done` and `uloop_run` (no Rust fd/timeout/process API yet; for those, drop down to `libubox_sys::*`).

## Cargo features

| Feature | Default | Effect |
|---------|---------|--------|
| `json`  | off     | Pulls in `libubox-sys/json`. Enables `BlobmsgBuf::format_json` and `BlobmsgBuf::add_json_str`. |

Build prerequisites are inherited from `libubox-sys` (CMake, json-c headers); see that crate's README for system-mode overrides and cross-compile notes.

MSRV: 1.90.

## License

Dual licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
