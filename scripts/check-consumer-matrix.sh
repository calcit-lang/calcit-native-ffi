#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

# shellcheck source=../consumer-pins.env
source "$repo_root/consumer-pins.env"

matrix_root="${CALCIT_NATIVE_FFI_MATRIX_DIR:-$(mktemp -d)}"
matrix_root_created="${CALCIT_NATIVE_FFI_MATRIX_DIR:+false}"
matrix_root_created="${matrix_root_created:-true}"
cargo_patch="patch.crates-io.calcit_native_ffi.path='$repo_root'"
native_version="$(awk -F'"' '/^version = "/ { print $2; exit }' "$repo_root/Cargo.toml")"
if [[ -z "$native_version" ]]; then
  echo "unable to read calcit_native_ffi package version" >&2
  exit 1
fi

cleanup() {
  if [[ "$matrix_root_created" == "true" ]]; then
    rm -rf "$matrix_root"
  else
    echo "consumer matrix kept at $matrix_root"
  fi
}
trap cleanup EXIT

run_category() {
  local category="$1"
  shift
  echo "::group::$category"
  if "$@"; then
    echo "::endgroup::"
  else
    local status=$?
    echo "::endgroup::"
    echo "::error title=$category::cross-consumer ABI smoke failed in $category"
    return "$status"
  fi
}

clone_pin() {
  local name="$1"
  local repository="$2"
  local revision="$3"
  local checkout="$matrix_root/$name"
  git init --quiet "$checkout"
  git -C "$checkout" remote add origin "$repository"
  git -C "$checkout" fetch --quiet --depth 1 origin "$revision"
  local fetched
  fetched="$(git -C "$checkout" rev-parse FETCH_HEAD)"
  if [[ "$fetched" != "$revision" ]]; then
    echo "$name resolved $fetched instead of pinned revision $revision" >&2
    return 1
  fi
  git -C "$checkout" checkout --quiet --detach "$revision"
}

checkout_consumers() {
  clone_pin calcit "$CALCIT_REPOSITORY" "$CALCIT_REVISION"
  clone_pin caps "$CAPS_REPOSITORY" "$CAPS_REVISION"
  clone_pin bindgen "$BINDGEN_REPOSITORY" "$BINDGEN_REVISION"
  clone_pin wss "$WSS_REPOSITORY" "$WSS_REVISION"
  clone_pin std "$STD_REPOSITORY" "$STD_REVISION"
  clone_pin regex "$REGEX_REPOSITORY" "$REGEX_REVISION"
}

build_core_host() {
  (
    cd "$matrix_root/calcit"
    cargo --config "$cargo_patch" update -p calcit_native_ffi --precise "$native_version"
    cargo --config "$cargo_patch" build --release --bin calcit
    cargo --config "$cargo_patch" test --lib ffi_abi::tests
    cargo --config "$cargo_patch" test --bin calcit async_callback_tests
  )
}

generated_consumer_smoke() {
  local generated="$matrix_root/generated"
  local fixture="$repo_root/tests/fixtures/generated-md5"
  local fixture_target="$matrix_root/generated-target"
  (
    cd "$matrix_root/bindgen"
    cargo run --quiet -- generate tests/fixtures/md5-interface.json --out "$generated" --backend rust
    cargo run --quiet -- check tests/fixtures/md5-interface.json --out "$generated" --backend rust
  )
  local bindings="$generated/rust/bindings.rs"
  test -f "$bindings"
  CALCIT_BINDINGS_PATH="$bindings" CARGO_TARGET_DIR="$fixture_target" \
    cargo --config "$cargo_patch" build --quiet --release --manifest-path "$fixture/Cargo.toml"

  local library
  case "$(uname -s)" in
    Darwin) library="$fixture_target/release/libcalcit_native_ffi_generated_md5_smoke.dylib" ;;
    Linux) library="$fixture_target/release/libcalcit_native_ffi_generated_md5_smoke.so" ;;
    *) echo "unsupported consumer-matrix platform: $(uname -s)" >&2; return 1 ;;
  esac

  (
    cd "$matrix_root/caps"
    cargo --config "$cargo_patch" build --quiet --bin caps
    ./target/debug/caps __verify-native "$library"
  )
  (
    cd "$matrix_root/calcit"
    ./target/release/calcit calcit/test.cirru eval \
      "assert= |5d41402abc4b2a76b9719d911017c592 $ &call-dylib-edn |$library |md5 |hello"
  )
}

blocking_consumer_smoke() {
  (
    cd "$matrix_root/std"
    cargo --config "$cargo_patch" build --quiet --release
    mkdir -p dylibs
    case "$(uname -s)" in
      Darwin) cp target/release/libcalcit_std.dylib dylibs/ ;;
      Linux) cp target/release/libcalcit_std.so dylibs/ ;;
    esac
    "$matrix_root/calcit/target/release/calcit" calcit.cirru analyze check-examples \
      --ns calcit.std.fs --def read-file-by-line!
  )
}

async_consumer_smoke() {
  (
    cd "$matrix_root/wss"
    cargo --config "$cargo_patch" build --quiet --release
    mkdir -p dylibs
    case "$(uname -s)" in
      Darwin) cp target/release/libcalcit_wss.dylib dylibs/ ;;
      Linux) cp target/release/libcalcit_wss.so dylibs/ ;;
    esac
    CALCIT_BIN="$matrix_root/calcit/target/release/calcit" bash scripts/check-wss-ffi.sh
  )
}

resource_consumer_smoke() {
  local trace_log="$matrix_root/regex-resource.log"
  (
    cd "$matrix_root/regex"
    cargo --config "$cargo_patch" build --quiet --release
    mkdir -p dylibs
    case "$(uname -s)" in
      Darwin) cp target/release/libcalcit_regex.dylib dylibs/ ;;
      Linux) cp target/release/libcalcit_regex.so dylibs/ ;;
    esac
    "$matrix_root/calcit/target/release/calcit" --trace-ffi calcit.cirru 2>&1 | tee "$trace_log"
  )
  grep -q 'resource-create' "$trace_log"
  grep -q 'resource-release' "$trace_log"
  grep -q 'Compiled-regex-methods-passed' "$trace_log"
}

mkdir -p "$matrix_root"
run_category revision-pins checkout_consumers
run_category symbol-layout build_core_host
run_category allocator-ownership-and-codec generated_consumer_smoke
run_category blocking-lifecycle blocking_consumer_smoke
run_category async-cancel-and-terminal-ordering async_consumer_smoke
run_category resource-lease-and-release resource_consumer_smoke

echo "cross-consumer ABI matrix passed"
