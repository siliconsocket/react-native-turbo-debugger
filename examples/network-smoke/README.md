# Local network and state smoke test

This example makes a Fetch GET and an XMLHttpRequest POST against a local fixture server. The Debug UI runs deterministic Zustand, Redux Toolkit, Jotai, and direct capture scenarios. Use the [network and state test runbook](../../docs/NETWORK_TEST_RUNBOOK.md) to install the package, run all four Debug cases, and inspect the Release artifacts.

## Network-only baseline (0.1.0)

The 0.1.0 tarball was installed in both template apps. Each capture shows the actual HTTP requests reaching the desktop from the app.

| Platform | RN 0.76.9 Legacy | RN 0.83.1 New Architecture |
| --- | --- | --- |
| iOS Simulator | [Network capture](../../docs/screenshots/network-smoke/ios-0.76.9.png) | [Network capture](../../docs/screenshots/network-smoke/ios-0.83.1.png) |
| Android emulator | [Network capture](../../docs/screenshots/network-smoke/android-0.76.9.png) | [Network capture](../../docs/screenshots/network-smoke/android-0.83.1.png) |

The iOS validation used Xcode 26.5 on an Intel Mac and an iOS 26.4 simulator. Both temporary template apps needed the local `fmt` 11.0.2 headers to use `FMT_USE_CONSTEVAL 0` to compile with this toolchain. Android ran on a Genymotion Galaxy A14 using ADB reverse for both ports.

Both Android Release APKs excluded the collector manifest entry and DEX classes. The Android apps also built Release with the package absent from `node_modules`. Neither iOS Release simulator binary contained observer symbols in the x86_64 or arm64 slice.
