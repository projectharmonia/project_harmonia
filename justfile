#!/usr/bin/env just --justfile

clean:
  cargo clean

raw-coverage $RUSTC_BOOTSTRAP="1" $LLVM_PROFILE_FILE=(justfile_directory() / "target/coverage/profile-%p.profraw") $RUSTFLAGS="-C instrument-coverage --cfg coverage":
  cargo test 

coverage *ARGS: clean raw-coverage && clean
  grcov target/coverage \
    --binary-path target/debug/ \
    --source-dir . \
    --excl-start "mod tests" \
    --excl-line "#\[" \
    --ignore "/*" \
    --ignore "src/ui*" \
    --ignore "*/main.rs" \
    --ignore "src/core/network/client.rs" \
    --ignore "src/core/network/server.rs" \
    --ignore "src/core/cli.rs" \
    --ignore "src/core/object/cursor_object.rs" \
    {{ ARGS }}
