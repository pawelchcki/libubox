#!/usr/bin/env bash
# Regenerate libubox-sys/src/bindings/pregenerated.rs from the vendored headers.
#
# Run on x86_64-unknown-linux-gnu. The output is portable across 32/64-bit
# Unix targets because libubox's public headers use uint32_t/uint64_t/size_t/
# time_t (no bare `long` in struct layouts) and `int` for fds.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
sys_crate="${repo_root}/libubox-sys"

cd "${repo_root}"

cargo build -p libubox-sys --features "bindgen json"

# Locate bindings.rs from the most recent libubox-sys build dir.
target_dir="${CARGO_TARGET_DIR:-target}"
mapfile -t candidates < <(find "${target_dir}" -path '*/build/libubox-sys-*/out/bindings.rs' -printf '%T@\t%p\n' \
                          | sort -nr)
if [[ ${#candidates[@]} -eq 0 ]]; then
    echo "could not locate generated bindings.rs in ${target_dir}/" >&2
    exit 1
fi
generated="$(printf '%s\n' "${candidates[0]}" | cut -f2-)"

dest="${sys_crate}/src/bindings/pregenerated.rs"
mkdir -p "$(dirname "${dest}")"

# rustfmt --emit files rewrites in place; copy first so we don't disturb the
# build artifact.
tmp="$(mktemp --suffix=.rs)"
trap 'rm -f "${tmp}"' EXIT
cp "${generated}" "${tmp}"
rustfmt --edition 2021 "${tmp}"

# Stamp the vendored-libubox SHA into the file header so reviewers can spot
# stale bindings if the submodule is bumped without re-running this script.
libubox_sha="$(git -C "${sys_crate}/vendor/libubox" rev-parse HEAD)"
header="/* libubox source: vendor/libubox @ ${libubox_sha} */
/* Regenerate with: tools/regen-bindings.sh (uses --features \"bindgen json\") */
"
# Insert the header after bindgen's own first-line banner.
awk -v h="${header}" 'NR==1 { print; printf "%s", h; next } { print }' "${tmp}" > "${tmp}.stamped"
mv "${tmp}.stamped" "${dest}"
rm -f "${tmp}"
trap - EXIT

echo "wrote ${dest}"
