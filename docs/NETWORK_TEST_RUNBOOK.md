# React Native network capture test runbook

Use this runbook to verify that locally installed Debug builds send real React Native requests to the macOS desktop. Test **RN 0.76.9 with Legacy Architecture** and **RN 0.83.1 with New Architecture** on both the iOS simulator and an Android emulator. Use disposable template apps so each result reflects a fresh installation of the npm tarball.

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
    npm install -D "$RNTD_WORK/siliconsocket-react-native-turbo-debugger-0.1.0.tgz"
  ) || break
done
```

In **both** apps, add the Debug-only Gradle and Podfile snippets in the [package setup](../packages/react-native-turbo-debugger/README.md). In `TurboLegacy/android/gradle.properties`, set `newArchEnabled=false`. Keep `newArchEnabled=true` in TurboModern.

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

**Android.** Run `adb devices -l` and use a device whose state is `device`. Replace `emulator-5554` with its serial and forward both ports before launching.

```sh
adb -s emulator-5554 reverse tcp:7410 tcp:7410
adb -s emulator-5554 reverse tcp:7411 tcp:7411
adb -s emulator-5554 reverse --list
npm run android -- --mode debug --deviceId emulator-5554
```

### Pass criteria for each case

1. The app shows `Network smoke: fetch=200 xhr=503`.
2. The desktop header identifies the connected app and platform. **Network Requests** shows these rows, including a duration for each.

   | Method | Path | HTTP status |
   | --- | --- | --- |
   | GET | `127.0.0.1/smoke-fetch` | 200 |
   | POST | `127.0.0.1/smoke-xhr` | 503 |

3. The POST path ends at `/smoke-xhr`. The example sends `?marker=omit-this-query` so the test can verify that query strings are omitted.
4. After terminating the app, the desktop shows **Disconnected** and keeps both rows visible.

The 503 is the fixture's expected response. Save a screenshot of the desktop for each case. [Reference captures and tested environment](../examples/network-smoke/README.md#debug-results)

## 4. Verify Release exclusion

Run these checks from each app directory after the Debug cases. A successful build and clean artifact inspection are both required.

**Android.** Build the APK and inspect its manifest and DEX packages with `apkanalyzer` from the Android SDK command-line tools.

```sh
(cd android && ./gradlew :app:assembleRelease)
apk=android/app/build/outputs/apk/release/app-release.apk
apkanalyzer manifest print "$apk" > release-manifest.txt
apkanalyzer dex packages --defined-only "$apk" > release-dex.txt
grep -E 'NetworkCaptureProvider|com\.siliconsocket\.rnturbodebugger' release-manifest.txt release-dex.txt
```

**iOS.** Build the simulator Release executable and inspect its symbols. `app_name` assumes the template directory and Xcode scheme share the same name.

```sh
app_name="${PWD##*/}"
xcodebuild -workspace "ios/$app_name.xcworkspace" -scheme "$app_name" \
  -configuration Release -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath ios/build/network-smoke-release CODE_SIGNING_ALLOWED=NO build

binary="ios/build/network-smoke-release/Build/Products/Release-iphonesimulator/$app_name.app/$app_name"
nm -arch all -a "$binary" > release-symbols.txt
grep -E 'RNTDRequestObserver|RNTDWriter|rntd_sendRequest|RNTurboDebugger' release-symbols.txt
```

Each `grep` should print nothing and exit with status 1. Confirm the analyzer or `nm` completed successfully before recording a pass. A diagnostic or an unreadable artifact makes the check inconclusive.

## Troubleshoot and report

| Symptom | Check |
| --- | --- |
| Desktop opens the Trailhead demo | Start it with `RNTD_LIVE=1`. |
| Desktop reports **Port unavailable** | Run `lsof -nP -iTCP:7410 -sTCP:LISTEN` and close the previous capture session. |
| The app cannot reach the fixture | Check `curl -i http://127.0.0.1:7411/smoke-fetch` on the Mac. On Android, check the selected serial and both ADB mappings. |
| The app shows both responses while the desktop stays waiting | Confirm the package path resolves with `node -p "require.resolve('@siliconsocket/react-native-turbo-debugger/package.json')"`. Check the Debug Gradle or Podfile link, rebuild, and relaunch after starting the desktop. |
| iOS build fails in `fmt` with a `consteval` error | Record the Xcode version. The [reference environment](../examples/network-smoke/README.md#debug-results) documents the workaround used for the temporary test apps. |

Report the debugger commit SHA, RN version, platform, simulator or emulator, and Xcode or Android SDK version. For each case, report **Debug capture** and **Release exclusion** as pass, fail, or blocked. Attach the desktop screenshot and the relevant fixture, build, or analyzer output for a failure.
