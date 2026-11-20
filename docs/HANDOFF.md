# Engineering handoff

React Native Turbo Debugger is in development on the `develop` branch. The current deliverable is an interactive macOS prototype built with Rust and GPUI. It opens a local Trailhead fixture by default. Live mode receives HTTP metadata from React Native Debug apps through the local npm package. The network test runbook covers iOS and Android on Legacy and New Architecture. [Network test runbook](NETWORK_TEST_RUNBOOK.md)

## Run and inspect

From the repository root, run `cargo run --locked` with Rust, Cargo and Xcode installed. On first launch, expect six events, two network requests and one Zustand action. Use **+ Demo event** to append a deterministic update. Restart the app to reset the session. The [README](../README.md) lists the launch variables, fixture scenarios and keyboard shortcuts.

## Where to make changes

| Area | Entry point | Behavior to preserve |
| --- | --- | --- |
| Session data | [Session types](../src/session.rs) and [demo source](../src/mock.rs) | Keep IDs stable and timestamps in elapsed milliseconds. Load the bundled fixture from the executable. |
| Capture and inspection | [Capture model](../src/app.rs) | Pausing pins the visible cursor while capture continues. Stopping preserves captured data for inspection. |
| Desktop shell | [Startup](../src/main.rs) and [workspace](../src/ui/mod.rs) | macOS owns the window controls. Panel expansion fills the workspace and preserves selection and filters on restore. |
| Instruments | [Console](../src/ui/console.rs) and [State](../src/ui/investigation.rs) | Match related events within the session by explicit correlation ID. Filter the JS and native channels independently. |
| Appearance | [Theme](../src/ui/theme.rs) and [tokens](../assets/tokens.json) | Keep code readable without color. Bundle fonts together with their licenses. |

## Acceptance walkthrough

1. Open **State** and select **selectTrail**. Expect revision 4 → 5, with `selectedTrail` changing from `null` to `"ridge-07"`. The related logs include the MapKit selection. The sync failure belongs to a separate operation.
2. In **Console**, select **POST /api/sync**. Expect HTTP 503, a duration of 212 ms and a related JS warning with `retryInMs` set to 5000.
3. Pause, append a demo event and resume. Confirm the cursor stays pinned during pause and advances on resume. Stop capture and inspect the recorded events.
4. Expand and restore each panel. Try **Esc** as well as **Restore**. At 1024 × 700, switch log channels and open the event details drawer. At 1280 × 800 and 1440 × 900, compare both views with the [window captures](../README.md#window-captures).
5. Enter fullscreen with the native green window button and exit. Confirm the original window size returns and the macOS controls remain visible and usable.

## Verification and next integration point

Run `cargo fmt --check`, `cargo test --locked` and `cargo clippy --locked --all-targets -- -D warnings`. The automated tests cover event correlation, retry metadata and cursor and selection behavior. Use the walkthrough for native window and panel interactions.

The local bridge projects mobile request events into the capture model. Future state integrations can use the same model to preserve pause behavior, stable selections, and explicit event correlation.
