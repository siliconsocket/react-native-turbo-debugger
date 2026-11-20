# Engineering handoff

React Native Turbo Debugger is in development on the `develop` branch. The macOS desktop uses Rust and GPUI and opens with the bundled Trailhead session. Live mode receives HTTP metadata and state transitions from the Debug-only React Native package. Version 0.2.0 is available as a local tarball and includes `sendState` and optional Zustand, Redux Toolkit, and Jotai adapters. [Package README](../packages/react-native-turbo-debugger/README.md)

The verified matrix covers React Native 0.76.9 Legacy and 0.83.1 New Architecture on both iOS and Android. It includes live state and Network capture and Release artifact inspection. [Test runbook and screenshots](NETWORK_TEST_RUNBOOK.md#verified-matrix-for-version-020)

## Run and inspect

From the repository root, run `cargo run --locked` with Rust, Cargo and Xcode installed. On first launch, expect six events, two network requests and one Zustand action. Use **+ Demo event** to append a deterministic update. Restart the app to reset the session. The [README](../README.md) lists the launch variables, fixture scenarios and keyboard shortcuts.

Start the desktop with `RNTD_LIVE=1 cargo run --locked` to listen on port 7410. Network and state arrive over the same connection. The desktop keeps the last session after disconnect and opens a new session when the app reconnects. [Contributor guide](../CONTRIBUTING.md)

## Where to make changes

| Area | Entry point | Current behavior |
| --- | --- | --- |
| Mobile state API | [Package entry point](../packages/react-native-turbo-debugger/index.js) and [module spec](../packages/react-native-turbo-debugger/specs/NativeRNTurboDebuggerState.ts) | Adapters send projected JSON through `sendState`. Legacy uses a Native Module and New Architecture uses a generated TurboModule. Android New Architecture links the generated provider into the Debug app. |
| Session data and wire protocol | [Session types](../src/session.rs), [bridge](../src/bridge.rs), and [demo source](../src/mock.rs) | IDs are stable within a session and timestamps are elapsed milliseconds. The bridge accepts version 1 Network clients and state transitions up to 16 KiB. Related events share an explicit correlation ID. |
| Capture and inspection | [Capture model](../src/app.rs) | Revisions are tracked per session, source, and store or atom. Pausing pins the visible cursor while capture continues. Stopping preserves captured data for inspection. |
| Desktop shell | [Startup](../src/main.rs) and [workspace](../src/ui/mod.rs) | macOS owns the window controls. Panel expansion fills the workspace and preserves selection and filters on restore. |
| Instruments | [Console](../src/ui/console.rs) and [State Investigation](../src/ui/investigation.rs) | The three source switches filter the state history. If a switch hides the selection, the inspector uses the newest visible transition or shows an empty state. |
| Appearance | [Theme](../src/ui/theme.rs), [tokens](../assets/tokens.json), and [code renderer](../src/ui/code.rs) | Snapshots, response bodies, and log arguments use selectable, Monokai-inspired JSON rendering. |

## Acceptance walkthrough

1. Open **State** and select **selectTrail**. Expect revision 4 → 5, with `selectedTrail` changing from `null` to `"ridge-07"`. The related logs include the MapKit selection. The sync failure belongs to a separate operation.
2. In **Console**, select **POST /api/sync**. Expect HTTP 503, a duration of 212 ms and a related JS warning with `retryInMs` set to 5000.
3. Pause, append a demo event and resume. Confirm the cursor stays pinned during pause and advances on resume. Stop capture and inspect the recorded events.
4. Expand and restore each panel. Try **Esc** as well as **Restore**. At 1024 × 700, switch log channels and open the event details drawer. At 1280 × 800 and 1440 × 900, compare both views with the [window captures](../README.md#window-captures).
5. Enter fullscreen with the native green window button and exit. Confirm the original window size returns and the macOS controls remain visible and usable.

## Verification

The Rust tests cover correlation, retry metadata, wire framing, state revisions, and cursor and selection behavior. The Trailhead walkthrough exercises the desktop without a mobile app. The runbook covers the four live mobile cases and the Release checks. [Desktop commands](../CONTRIBUTING.md#check-desktop-changes) · [Mobile test runbook](NETWORK_TEST_RUNBOOK.md)
