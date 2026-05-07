use std::env;
use std::path::PathBuf;

const ENV_LIBUBOX_DIR: &str = "LIBUBOX_DIR";
const ENV_LIBUBOX_INCLUDE_DIR: &str = "LIBUBOX_INCLUDE_DIR";
const ENV_LIBUBOX_LIB_DIR: &str = "LIBUBOX_LIB_DIR";
const ENV_LIBUBOX_STATIC: &str = "LIBUBOX_STATIC";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"))
}

fn resolve_system() -> Option<(PathBuf, PathBuf)> {
    let prefix = env::var_os(ENV_LIBUBOX_DIR).map(PathBuf::from);
    let inc = env::var_os(ENV_LIBUBOX_INCLUDE_DIR).map(PathBuf::from);
    let lib = env::var_os(ENV_LIBUBOX_LIB_DIR).map(PathBuf::from);
    if prefix.is_none() && inc.is_none() && lib.is_none() {
        return None;
    }
    let include = inc
        .or_else(|| prefix.as_ref().map(|p| p.join("include")))
        .expect("LIBUBOX_INCLUDE_DIR or LIBUBOX_DIR must be set");
    let lib = lib
        .or_else(|| prefix.as_ref().map(|p| p.join("lib")))
        .expect("LIBUBOX_LIB_DIR or LIBUBOX_DIR must be set");
    Some((include, lib))
}

fn build_vendored() -> (PathBuf, PathBuf) {
    let src = manifest_dir().join("vendor/libubox");
    assert!(
        src.join("CMakeLists.txt").exists(),
        "vendor/libubox is missing. Run: git submodule update --init --recursive"
    );
    // Cargo watches this directory recursively, so edits to any .c/.h/CMakeLists
    // under vendor/libubox trigger a rebuild. Only emit on the vendored path —
    // in system mode (LIBUBOX_DIR set) the vendor tree is not a build input.
    println!("cargo:rerun-if-changed=vendor/libubox");

    let dst = cmake::Config::new(&src)
        .define("BUILD_LUA", "OFF")
        .define("BUILD_EXAMPLES", "OFF")
        // CMake >= 4 dropped policy-<3.5 compatibility; libubox declares 3.13.
        .env("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        // libubox unconditionally adds -Werror; modern compilers trip on it.
        .cflag("-Wno-error")
        .build();

    (dst.join("include"), dst.join("lib"))
}

fn copy_pregenerated() {
    let src = manifest_dir().join("src/bindings/pregenerated.rs");
    let dst = out_dir().join("bindings.rs");
    println!("cargo:rerun-if-changed=src/bindings/pregenerated.rs");
    std::fs::copy(&src, &dst).expect(
        "copy src/bindings/pregenerated.rs — missing? Build with `--features bindgen` to \
         generate, or run tools/regen-bindings.sh",
    );
}

#[cfg(feature = "bindgen")]
fn run_bindgen(include_dir: &std::path::Path) {
    const ALLOW_ITEMS: &str = "blob.*|blobmsg.*|uloop.*|avl.*|kvlist.*|vlist.*|ulog.*|usock.*|md5.*|runqueue.*|ustream.*|safe_list.*";

    let header = manifest_dir().join("wrapper.h");

    let mut builder = bindgen::Builder::default()
        .header(header.to_string_lossy().into_owned())
        .clang_arg(format!("-I{}", include_dir.display()))
        .layout_tests(false)
        .derive_default(true)
        .generate_comments(false)
        .blocklist_type("FILE")
        .blocklist_type("fpos_t")
        .blocklist_type("_IO_.*")
        .blocklist_type("max_align_t")
        .allowlist_function(ALLOW_ITEMS)
        .allowlist_type(ALLOW_ITEMS)
        .allowlist_var("BLOB.*|BLOBMSG.*|ULOOP.*|AVL.*|ULOG.*|USOCK.*");

    #[cfg(feature = "json")]
    {
        builder = builder
            .clang_arg("-DLIBUBOX_SYS_WITH_JSON=1")
            .allowlist_function("blobmsg_.*_json.*|json_script_.*")
            .allowlist_type("json_script_.*|json_call|json_handler");
    }

    let bindings = builder.generate().expect("bindgen failed");
    bindings
        .write_to_file(out_dir().join("bindings.rs"))
        .expect("write bindings.rs");
}

fn main() {
    for var in [
        ENV_LIBUBOX_DIR,
        ENV_LIBUBOX_INCLUDE_DIR,
        ENV_LIBUBOX_LIB_DIR,
        ENV_LIBUBOX_STATIC,
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "json")]
    {
        // pkg_config::probe consults these; without rerun guards, cargo will
        // reuse a stale link-search when the user changes their environment.
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS");
    }

    // docs.rs short-circuit: just stage pregenerated bindings, don't link/build.
    if env::var_os("DOCS_RS").is_some() {
        copy_pregenerated();
        return;
    }

    // The env var is additive to the `static` feature: it can enable static
    // linking but cannot disable it once the feature is on.
    let want_static = cfg!(feature = "static")
        || env::var(ENV_LIBUBOX_STATIC)
            .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"));

    let (include_dir, lib_dir) = resolve_system().unwrap_or_else(build_vendored);

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    let kind = if want_static { "static" } else { "dylib" };
    // Order: dependents before deps. Static archives are scanned once each,
    // so blobmsg_json/json_script (which call libubox symbols) must precede
    // ubox; with shared libs the order is harmless.
    #[cfg(feature = "json")]
    {
        println!("cargo:rustc-link-lib={}=blobmsg_json", kind);
        // libubox upstream only builds json_script as SHARED, so we always
        // dynamic-link it even when `static` is requested.
        println!("cargo:rustc-link-lib=dylib=json_script");
    }
    println!("cargo:rustc-link-lib={}=ubox", kind);

    #[cfg(feature = "json")]
    pkg_config::Config::new()
        .atleast_version("0.13")
        .probe("json-c")
        .expect("`json` feature requires json-c (pkg-config + libjson-c-dev)");

    if cfg!(target_os = "linux") {
        // libubox conditionally links rt; force-link to be safe across libcs.
        println!("cargo:rustc-link-lib=rt");
    }

    // Expose the include dir to downstream sys consumers (e.g. ubus-sys) via
    // DEP_UBOX_INCLUDE. We don't expose `cargo:root` because deriving a
    // sensible root from `LIBUBOX_LIB_DIR` (e.g. /usr/lib/x86_64-linux-gnu)
    // is not robust.
    println!("cargo:include={}", include_dir.display());

    #[cfg(feature = "bindgen")]
    run_bindgen(&include_dir);
    #[cfg(not(feature = "bindgen"))]
    copy_pregenerated();
}
