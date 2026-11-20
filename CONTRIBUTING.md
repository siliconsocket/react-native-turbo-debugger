# Contributing

## Run the desktop

On macOS with Rust, Cargo, and Xcode installed, open the desktop with the bundled Trailhead session.

```sh
cargo run --locked
```

The [project README](README.md) describes the demo scenarios, live mode, and keyboard shortcuts. The [engineering handoff](docs/HANDOFF.md) maps the codebase and explains how capture behaves today.

## Check desktop changes

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The Trailhead fixture gives desktop changes a reproducible starting point. The compact layout is available with `RNTD_SIZE=1024x700 cargo run --locked`.

## Try mobile changes

The React Native package is installed from a local npm tarball. The [package README](packages/react-native-turbo-debugger/README.md) covers native setup and state adapter examples. The [test runbook](docs/NETWORK_TEST_RUNBOOK.md) walks through live capture and Release builds on iOS and Android.

A short reproduction, expected behavior, and any relevant test output or screenshots help other contributors pick up an issue or review a change.
