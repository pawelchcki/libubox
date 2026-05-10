//! Build a small blobmsg describing the host (uname/uptime/loadavg) and dump
//! it as JSON. Mirrors the shape of an OpenWrt `system info` ubus response,
//! without actually talking to ubusd. Run with `--features json`.
//!
//!     cargo run --example sysinfo --features json
//!
//! Cross-built for an OpenWrt aarch64 router this also exercises the
//! libubox-sys/libubox stack on its native target.

use libubox::{BlobError, BlobmsgBuf};
use std::ffi::CString;
use std::fs;

fn read_first_line(path: &str) -> std::io::Result<String> {
    Ok(fs::read_to_string(path)?
        .lines()
        .next()
        .unwrap_or("")
        .to_owned())
}

fn parse_uptime() -> (u64, f64) {
    let raw = fs::read_to_string("/proc/uptime").unwrap_or_default();
    let mut it = raw.split_ascii_whitespace();
    let up = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let idle = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0_f64);
    (up as u64, idle)
}

fn parse_loadavg() -> [f64; 3] {
    let raw = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut it = raw.split_ascii_whitespace();
    [
        it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
        it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
        it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
    ]
}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("nul in input")
}

fn build() -> Result<BlobmsgBuf, BlobError> {
    let mut buf = BlobmsgBuf::new()?;

    let kernel = read_first_line("/proc/sys/kernel/osrelease").unwrap_or_default();
    let hostname = read_first_line("/proc/sys/kernel/hostname").unwrap_or_default();
    let (uptime, _idle) = parse_uptime();
    let load = parse_loadavg();

    buf.add_string(c"hostname", &cstr(&hostname))?;
    buf.add_string(c"kernel", &cstr(&kernel))?;
    buf.add_u64(c"uptime", uptime)?;
    buf.add_array(c"load", |arr| {
        for v in load {
            arr.add_double(c"", v)?;
        }
        Ok(())
    })?;
    Ok(buf)
}

fn main() {
    let buf = build().expect("build blobmsg");

    #[cfg(feature = "json")]
    println!("{}", buf.format_json());
    #[cfg(not(feature = "json"))]
    {
        let root = buf.root();
        println!("blobmsg root: type_id={:?}, len={}", root.type_id(), root.raw().len());
        for child in root.iter() {
            println!(
                "  {:?}: type={:?}, payload={} bytes",
                child.name(),
                child.type_id(),
                child.payload().len()
            );
        }
        eprintln!("(rebuild with --features json for JSON output)");
    }
}
