# React Native Turbo Debugger

Development-only network and state capture for React Native on the iOS simulator and Android. Native collectors run before JavaScript and send HTTP metadata to the desktop on loopback port 7410. Requests appear in **Network Requests** with method, host and path, status, and time to response headers. Capture leaves the request body and response stream available to the application. Network events omit headers, query strings, fragments, and bodies. Apps explicitly register the state they want to send.

## Install from a local tarball

From this repository's root:

```sh
npm pack ./packages/react-native-turbo-debugger --pack-destination /tmp
```

From the React Native app:

```sh
npm install -D /tmp/siliconsocket-react-native-turbo-debugger-0.2.0.tgz
```

Capture starts from the native Debug build. Add the native dependency for each target platform below, then rebuild. Apps can keep their existing JavaScript networking code.

### Android

Add this to the app's `android/settings.gradle`:

```groovy
def turboDebuggerPackage = providers.exec {
    workingDir(rootDir)
    commandLine('node', '-p', "try { require.resolve('@siliconsocket/react-native-turbo-debugger/package.json') } catch (e) { if (e.code !== 'MODULE_NOT_FOUND') throw e; '' }")
}.standardOutput.asText.get().trim()

if (turboDebuggerPackage) {
    include ':rn-turbo-debugger'
    project(':rn-turbo-debugger').projectDir = new File(new File(turboDebuggerPackage).parentFile, 'android')
}
```

Add this inside `dependencies` in `android/app/build.gradle`. Use the equivalent Debug configuration for flavored builds.

```groovy
if (findProject(':rn-turbo-debugger') != null) {
    debugImplementation project(':rn-turbo-debugger')
}
```

Register the state module from a Debug source set. In these Kotlin examples, replace `com.example` with the app's package. Call `addTurboDebugger()` inside the existing `PackageList(this).packages.apply { ... }` block in `MainApplication.kt`.

`android/app/src/debug/java/com/example/TurboDebugger.kt`

```kotlin
package com.example

import com.facebook.react.ReactPackage
import com.siliconsocket.rnturbodebugger.StateCapturePackage

internal fun MutableList<ReactPackage>.addTurboDebugger() {
    add(StateCapturePackage())
}
```

`android/app/src/release/java/com/example/TurboDebugger.kt`

```kotlin
package com.example

import com.facebook.react.ReactPackage

internal fun MutableList<ReactPackage>.addTurboDebugger() {}
```

#### Android New Architecture

Link the generated TurboModule provider into the Debug app's native entry point. If the app still uses React Native's default CMake and JNI files, copy them from the installed version into `android/app/src/main/jni`.

```sh
mkdir -p android/app/src/main/jni
cp node_modules/react-native/ReactAndroid/cmake-utils/default-app-setup/CMakeLists.txt android/app/src/main/jni/
cp node_modules/react-native/ReactAndroid/cmake-utils/default-app-setup/OnLoad.cpp android/app/src/main/jni/
```

Merge this into `android/app/build.gradle`.

```groovy
def debuggerProject = findProject(':rn-turbo-debugger')

android {
    externalNativeBuild {
        cmake {
            path 'src/main/jni/CMakeLists.txt'
        }
    }

    buildTypes {
        debug {
            if (debuggerProject != null) {
                externalNativeBuild {
                    cmake {
                        arguments "-DRNTD_ANDROID_DIR=${debuggerProject.projectDir}"
                    }
                }
            }
        }
    }
}

if (debuggerProject != null) {
    tasks.configureEach {
        if (name.startsWith('configureCMake') && name.contains('Debug')) {
            dependsOn "${debuggerProject.path}:generateCodegenArtifactsFromSchema"
        }
    }
}
```

At the end of the app's `CMakeLists.txt`, include the debugger after `ReactNative-application.cmake`.

```cmake
if(DEFINED RNTD_ANDROID_DIR)
  include("${RNTD_ANDROID_DIR}/state-codegen.cmake")
endif()
```

In `OnLoad.cpp`, add the header with the existing includes and call the provider inside `javaModuleProvider`, before the existing lookup logic.

```cpp
#ifdef RNTD_STATE_ENABLED
#include <RNTurboDebuggerStateSpec.h>
#endif
```

```cpp
#ifdef RNTD_STATE_ENABLED
  if (auto module = RNTurboDebuggerStateSpec_ModuleProvider(name, params)) {
    return module;
  }
#endif
```

#### Android connection

Forward the emulator or USB-connected Android device's port to the Mac:

```sh
adb reverse tcp:7410 tcp:7410
```

The package installs a factory based on React Native's standard OkHttp builder. If the app installs a custom factory, call `NetworkCapture.instrument(builder)` on its existing builder from debug-only code before building the client. The same helper can instrument a separate OkHttp or Ktor OkHttp client.

### iOS simulator

Add the local pod inside the React Native app target in `ios/Podfile`:

```ruby
debugger_package = Pod::Executable.execute_command('node', ['-p',
  "try { require.resolve('@siliconsocket/react-native-turbo-debugger/package.json') } catch (e) { if (e.code !== 'MODULE_NOT_FOUND') throw e; '' }"]).strip
unless debugger_package.empty?
  pod 'RNTurboDebugger', :path => File.join(File.dirname(debugger_package), 'ios'),
      :configurations => ['Debug']
end
```

Run `pod install` with your app’s architecture flag. The pod registers the state module and generates its native bindings for New Architecture. The package observes React Native's HTTP handler delegate and forwards every callback to the original delegate. The app's URL session still handles request bodies, caching, redirects and cookies. Capture in this version targets the iOS simulator and requests handled by React Native's built-in HTTP handler.

## Capture state

`sendState` and the three adapter entry points are typed. Use `__DEV__` when connecting capture in the consuming app. Each adapter accepts a projection so the app can choose which fields leave the device. Zustand, Redux Toolkit, and Jotai are optional dependencies. Install the ones your app uses.

### Send a transition directly

```ts
import {sendState} from '@siliconsocket/react-native-turbo-debugger';

if (__DEV__) {
  sendState({
    source: 'manual',
    store: 'onboarding',
    action: 'Advance',
    before: {step: 1},
    after: {step: 2},
    payload: {step: 2},
    correlationId: 'onboarding-42',
  });
}
```

`source`, `store`, `action`, `before`, and `after` are required. `payload` and `correlationId` are optional. Capture discards unserializable inputs and transitions larger than 16 KiB in UTF-8. The inspector links events in a session when both carry the same explicit correlation ID.

### Zustand

```ts
import {attachZustand} from '@siliconsocket/react-native-turbo-debugger/zustand';

const detach = __DEV__
  ? attachZustand(trailStore, {
      name: 'trailStore',
      project: state => ({zoom: state.zoom}),
    })
  : undefined;

// When the store is disposed:
detach?.();
```

The adapter uses the store's `subscribe` method. Add `describe(after, before)` to supply an action label, projected payload, or correlation ID. The default label is **State changed**.

### Redux Toolkit

```ts
import {configureStore} from '@reduxjs/toolkit';
import {createReduxStateMiddleware} from '@siliconsocket/react-native-turbo-debugger/redux';

const store = configureStore({
  reducer: rootReducer,
  middleware: getDefault => {
    const defaults = getDefault();
    return __DEV__
      ? defaults.concat(createReduxStateMiddleware<RootState>({
          name: 'rootStore',
          project: state => ({zoom: state.trails.zoom}),
        }))
      : defaults;
  },
});
```

The middleware records the action type and the projected state on each side of dispatch. `describe(action)` can supply metadata, including an explicitly selected payload. Dispatch keeps its return value and errors.

### Jotai

```ts
import {attachJotaiAtom} from '@siliconsocket/react-native-turbo-debugger/jotai';

const detach = __DEV__
  ? attachJotaiAtom(jotaiStore, zoomAtom, {
      name: 'zoomAtom',
      project: zoom => ({zoom}),
    })
  : undefined;

// When the atom is disposed:
detach?.();
```

Register each atom you want to observe on the store your app uses. For an async atom, the snapshots show `pending`, `resolved` with the projected value, or `rejected` as a status. A promise that has been replaced is ignored when it settles.

## View a live session

Run the desktop from the repository root:

```sh
RNTD_LIVE=1 cargo run --locked
```

Launch the Debug mobile app and make a request. The desktop shows **Network Requests** and a preview of the selected state diff in **Console**. Open **State Investigation** to select a transition and view **Diff**, **Next**, **Previous**, or **Raw**. The Zustand, Redux, and Jotai switches filter their histories while mobile capture continues. **Pause** freezes the inspection cursor. The desktop keeps the last session visible on disconnect and starts a new session on reconnect.
