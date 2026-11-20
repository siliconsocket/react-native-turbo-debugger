# React Native Turbo Debugger

**Status: In development.** The macOS desktop supports live network capture from React Native Debug builds and includes a bundled demo session. The development branch is `develop`. [Engineering handoff](docs/HANDOFF.md)

A native Rust and GPUI desktop prototype for inspecting a captured React Native session. It opens with six bundled Trailhead events from an iPhone 16 Pro.

## Run on macOS

Install Rust with Cargo and select a full Xcode installation. The first build downloads dependencies. GPUI compiles its Metal shaders at runtime. Fonts, theme tokens and demo data are embedded in the executable.

```sh
cargo run --locked
```

## Inspect a React Native app

Install the debug package from a local npm tarball, configure the app's Debug build and start the desktop with `RNTD_LIVE=1 cargo run --locked`. [Package setup](packages/react-native-turbo-debugger/README.md)

To reproduce the four iOS and Android cases and verify that Release builds exclude the collector, follow the [network test runbook](docs/NETWORK_TEST_RUNBOOK.md).

## Inspect the demo

- **Expand** fills the workspace with the selected panel. **Restore** or **Esc** returns to the split layout, keeping the current selection and filters.
- **Console** shows requests, the selected Zustand action, independent JS and native log streams, and a timeline. Click a request to inspect its response and correlated logs. Click the level label to cycle the filter, type to narrow a stream, or expand an entry to inspect its arguments.
- **State** shows action history, Diff, Next, Previous and Raw views, event details and related logs. Use the arrow keys to change actions. Related events require a matching correlation ID within the session.
- **+ Demo event** appends a deterministic Zustand update. **Pause** or a click on the time ruler pins the inspection cursor. Add another event, then **Resume** to see it. **Stop** ends capture for the current run and preserves inspection.
- **⌘1 / ⌘2** switches between Console and State. In compact windows, **⌘J / ⌘N** selects the JS or native channel and **⌘D** toggles the State details drawer. Tab moves focus between controls.

### Demo scenarios

```sh
RNTD_SCENARIO=pending cargo run --locked
RNTD_SCENARIO=long RNTD_VIEW=state cargo run --locked
RNTD_SIZE=1024x700 cargo run --locked
```

Available scenarios are `empty`, `disconnected`, `pending` and `long`. The default session exercises the missing app icon, missing action payload and HTTP 503 response. All times are elapsed session time.

## Project layout

| Module | Responsibility |
| --- | --- |
| `session` | Typed session and event data, log channels and correlation |
| `mock` | Bundled fixture and deterministic appended events |
| `app` | Capture lifecycle, cursor, visible events and selection |
| `ui` | GPUI window content, console, investigation and shared theme |

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Window captures

| Window | Session Console | State Investigation |
| --- | --- | --- |
| 1440 × 900 | [Console](docs/screenshots/console-1440x900.png) | [State](docs/screenshots/state-1440x900.png) |
| 1280 × 800 | [Console](docs/screenshots/console-1280x800.png) | [State](docs/screenshots/state-1280x800.png) |

The green macOS window button enters and exits native fullscreen. The verified round trip went from 1280 × 800 to 2560 × 1440 and restored the original window size with its native controls visible. [Fullscreen capture](docs/screenshots/fullscreen.png) · [Restored window](docs/screenshots/restored-1280x800.png)

The bundled Barlow Condensed and IBM Plex fonts are distributed with their licenses in `assets/fonts`.
