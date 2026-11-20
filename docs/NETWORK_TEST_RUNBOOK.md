# React Native network and state capture test runbook

Use this runbook to verify that locally installed Debug builds send React Native HTTP events and state transitions to the macOS desktop. Test **RN 0.76.9 with Legacy Architecture** and **RN 0.83.1 with New Architecture** on both the iOS simulator and an Android emulator. Use disposable template apps so each result reflects a fresh installation of the npm tarball.

## 1. Prepare the test apps

Use a Mac with Rust and Cargo, a full Xcode installation with an iOS simulator runtime, Node.js and npm, CocoaPods, and an Android SDK and JDK compatible with the two React Native templates. Confirm that `node`, `cargo`, `pod`, `xcodebuild` and `adb` are available in the shell.

From the debugger repository root, build the local tarball and set up an isolated directory for both apps.

```sh
export RNTD_REPO="$(pwd -P)"
export RNTD_WORK="$(mktemp -d "${TMPDIR:-/tmp}/rntd-smoke.XXXXXX")"
npm pack ./packages/react-native-turbo-debugger --pack-destination "$RNTD_WORK"
cd "$RNTD_WORK"
npx @react-native-community/cli@15.0.1 init TurboLegacy --version 0.76.9
npx @react-native-community/cli@20.0.0 init TurboModern --version 0.83.1

for app in TurboLegacy TurboModern; do
  cp "$RNTD_REPO/examples/network-smoke/App.tsx" "$RNTD_WORK/$app/App.tsx"
  (
    cd "$RNTD_WORK/$app" &&
    npm install -D "$RNTD_WORK/siliconsocket-react-native-turbo-debugger-0.2.0.tgz" zustand@5 @reduxjs/toolkit@2 jotai@2
  ) || break
done
```

In **both** apps, add the Debug-only Gradle linkage, Android package registration, and Podfile integration in the [package setup](../packages/react-native-turbo-debugger/README.md). In `TurboLegacy/android/gradle.properties`, set `newArchEnabled=false`. Keep `newArchEnabled=true` in TurboModern and add the New Architecture CMake and JNI integration from the package setup.

```sh
(cd "$RNTD_WORK/TurboLegacy/ios" && RCT_NEW_ARCH_ENABLED=0 pod install)
(cd "$RNTD_WORK/TurboModern/ios" && pod install)
```

## 2. Start the local services

Run these in separate terminals from the debugger repository root. The desktop listens on TCP **7410**. The fixture HTTP server listens on **7411** and prints each request it receives.

```sh
node examples/network-smoke/server.mjs
```

```sh
RNTD_LIVE=1 cargo run --locked
```

The desktop should show **Waiting for app**. Check the fixture from the Mac with `curl -i http://127.0.0.1:7411/smoke-fetch`. Expect HTTP 200. This health check appears in the fixture terminal.

## 3. Run the four Debug cases

Run one app at a time. For each case, run commands from the matching app directory under `RNTD_WORK`. In a separate terminal for that same directory, run `npm start` to serve the JavaScript bundle. Stop the current app and Metro before switching template apps so each native build loads its own bundle.

| Case | Directory | Target |
| --- | --- | --- |
| Legacy iOS | `TurboLegacy` | iOS simulator |
| New Architecture iOS | `TurboModern` | iOS simulator |
| Legacy Android | `TurboLegacy` | Android emulator |
| New Architecture Android | `TurboModern` | Android emulator |

**iOS.** Run `xcrun simctl list devices booted` to identify the simulator. Replace the simulator name in the command with yours.

```sh
npm run ios -- --mode Debug --simulator "iPhone 17 Pro"
```

**Android.** Run `adb devices -l` and use a device whose state is `device`. Replace `emulator-5554` with its serial and forward the debugger, fixture, and Metro ports before launching.

```sh
adb -s emulator-5554 reverse tcp:7410 tcp:7410
adb -s emulator-5554 reverse tcp:7411 tcp:7411
adb -s emulator-5554 reverse tcp:8081 tcp:8081
adb -s emulator-5554 reverse --list
npm run android -- --mode debug --deviceId emulator-5554
```

### Network and state checks for each case

1. The app shows `Network smoke: fetch=200 xhr=503`.
2. The desktop header identifies the connected app and platform. **Network Requests** shows these rows, including a duration for each.

   | Method | Path | HTTP status |
   | --- | --- | --- |
   | GET | `127.0.0.1/smoke-fetch` | 200 |
   | POST | `127.0.0.1/smoke-xhr` | 503 |

3. The POST path ends at `/smoke-xhr`. The example sends `?marker=omit-this-query` so the test can verify that query strings are omitted.
4. Run each scenario separately by pressing its button on the mobile app. Verify each new row before running the next scenario. In the desktop, open **State Investigation** and check the source, store or atom, action, **Diff**, and **Raw** against the table. In **Raw**, verify the lowercase source identifier and the `before` and `after` objects. The smoke app wraps its state calls in a `__DEV__` UI guard. See the package README for examples that register each adapter under `__DEV__`. [Package setup and examples](../packages/react-native-turbo-debugger/README.md)

   | Button | Source | Store or atom | Action | Before → after |
   | --- | --- | --- | --- | --- |
   | Run Zustand | `zustand` | `zustandCounter` | `State changed` | `{"count":0}` → `{"count":1}` |
   | Run Redux | `redux` | `reduxCounter` | `counter/increment` | `{"count":10}` → `{"count":12}` |
   | Run Jotai | `jotai` | `jotaiCounter` | `Atom changed` | `{"count":20}` → `{"count":23}` |
   | Run direct capture | `manual` | `directCounter` | `Set count` | `{"count":40}` → `{"count":45}` |

5. The direct transition also carries `payload: {"step":5}` and `correlationId: "direct-smoke"`. Confirm **Next** and **Previous** show the corresponding snapshots. Return to **Console** and verify the selected state diff appears alongside GET 200 and POST 503.
6. In a fresh session, press **Run all three**. At 1024 × 700, turn each switch off and back on independently. Only that source should disappear and return. When a switch hides the selected row, the newest visible row takes its place. With all three off and no direct events, the history and inspector show **No state transitions**.
7. Click **Pause** and press **Run direct capture**. Toggle a source while paused. The newly sent direct event should appear only after **Resume**. Press **Run discard checks** and confirm that a new `directCounter` event arrives without any `discardCounter` event. The button sends circular and oversized inputs through JavaScript, then malformed and oversized JSON directly to the native entry point before sending a valid transition.
8. Press **Run async Jotai**. Look for `pending`, `resolved` with count 34, and `rejected` in **Raw**. There should be no resolution with count 99, which belongs to the replaced promise.
9. Terminate the app. The desktop should show **Disconnected** and retain the session. Relaunch the app to start a new session, and press the same scenario again. Its first transition should show revision 0 → 1.

The 503 is the fixture's expected response. Capture a desktop screenshot for each matrix row.

## 4. Verify Release exclusion

Run these checks from each app directory after the Debug cases. A successful build and clean artifact inspection are both required.

**Android.** Build the APK and inspect its manifest and DEX packages with `apkanalyzer` from the Android SDK command-line tools. Check the New Architecture app’s JNI binary for the generated provider as well.

```sh
(cd android && ./gradlew :app:assembleRelease)
apk=android/app/build/outputs/apk/release/app-release.apk
apkanalyzer manifest print "$apk" > release-manifest.txt
apkanalyzer dex packages --defined-only "$apk" > release-dex.txt
rg 'NetworkCaptureProvider|StateCapturePackage|NativeRNTurboDebuggerStateSpec|com[./]siliconsocket[./]rnturbodebugger' release-manifest.txt release-dex.txt

# New Architecture
unzip -p "$apk" 'lib/*/libappmodules.so' > release-appmodules.so
strings release-appmodules.so > release-appmodules.txt
rg 'RNTurboDebuggerState' release-appmodules.txt
```

**iOS.** Build the simulator Release executable and inspect its symbols. `app_name` assumes the template directory and Xcode scheme share the same name.

```sh
app_name="${PWD##*/}"
xcodebuild -workspace "ios/$app_name.xcworkspace" -scheme "$app_name" \
  -configuration Release -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath ios/build/network-smoke-release CODE_SIGNING_ALLOWED=NO build

binary="ios/build/network-smoke-release/Build/Products/Release-iphonesimulator/$app_name.app/$app_name"
nm -arch all -a "$binary" > release-symbols.txt
rg 'RNTDRequestObserver|RNTDWriter|rntd_sendRequest|RNTDEnqueueState|RNTurboDebugger' release-symbols.txt
```

Each `rg` should print nothing and exit with status 1. Confirm the analyzer or `nm` completed successfully before recording a pass. A diagnostic or an unreadable artifact makes the check inconclusive.

## Verified matrix for version 0.2.0

Each row used an installed 0.2.0 tarball. Each adapter and direct capture delivered its expected action and snapshots in **Diff** and **Raw**. Network capture showed GET 200 and POST 503 alongside state in the same session.

| React Native | Platform | Network | Zustand | Redux Toolkit | Jotai | Direct | Release exclusion | Desktop capture |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0.76.9 Legacy | iPhone 17 Pro simulator | Pass | Pass | Pass | Pass | Pass | Pass | [Redux Diff](screenshots/state-smoke/ios-0.76.9.png) |
| 0.83.1 New Architecture | iPhone 17 Pro simulator | Pass | Pass | Pass | Pass | Pass | Pass | [Zustand Raw](screenshots/state-smoke/ios-0.83.1-raw.png), [Console](screenshots/state-smoke/ios-0.83.1-console.png) |
| 0.76.9 Legacy | Galaxy A14 emulator | Pass | Pass | Pass | Pass | Pass | Pass | [Console](screenshots/state-smoke/android-0.76.9.png) |
| 0.83.1 New Architecture | Galaxy A14 emulator | Pass | Pass | Pass | Pass | Pass | Pass | [Raw with source filters](screenshots/state-smoke/android-0.83.1-raw.png) |

Xcode 26.5 and the iOS 26.4 simulator ran on an Intel Mac. Both disposable iOS apps used the local `fmt` workaround from the network baseline. Android ran on Genymotion with Android 15 and JDK 17. The Legacy app used compile SDK 35 and the New Architecture app used compile SDK 36.

The compact 1024 × 700 window showed all three switches and respected the pinned cursor. The direct capture remained available after malformed and oversized events were discarded. The Jotai async scenario produced pending, resolved, pending, and rejected snapshots after reconnect. The replaced promise's count of 99 never appeared. [Jotai rejection in Raw](screenshots/state-smoke/ios-0.83.1-async.png)

## Troubleshoot and report

| Symptom | Check |
| --- | --- |
| Desktop opens the Trailhead demo | Start it with `RNTD_LIVE=1`. |
| Desktop reports **Port unavailable** | Run `lsof -nP -iTCP:7410 -sTCP:LISTEN` and close the previous capture session. |
| The app cannot reach the fixture | Check `curl -i http://127.0.0.1:7411/smoke-fetch` on the Mac. On Android, check the selected serial and both ADB mappings. |
| The app shows both responses while the desktop stays waiting | Confirm the package path resolves with `node -p "require.resolve('@siliconsocket/react-native-turbo-debugger/package.json')"`. Check the Debug Gradle or Podfile link, rebuild, and relaunch after starting the desktop. |
| iOS build fails in `fmt` with a `consteval` error | Record the Xcode version. The [reference environment](../examples/network-smoke/README.md#network-only-baseline-010) documents the workaround used for the temporary test apps. |

Report the debugger commit SHA, RN version, platform, simulator or emulator, and Xcode or Android SDK version. For each case, report **Network**, **Zustand**, **Redux Toolkit**, **Jotai**, **direct capture**, and **Release exclusion** as pass, fail, or blocked. Attach the desktop screenshot and the relevant fixture, build, or analyzer output for a failure.
