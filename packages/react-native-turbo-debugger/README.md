# React Native Turbo Debugger

Development-only network capture for iOS and Android React Native apps. Native collectors run before JavaScript and send HTTP metadata to the desktop on loopback port 7410. Requests appear in **Network Requests** with method, host and path, status, and time to response headers. Capture leaves the request body and response stream available to the application. Events omit headers, query strings, fragments, and bodies.

## Install from a local tarball

From this repository's root:

```sh
npm pack ./packages/react-native-turbo-debugger --pack-destination /tmp
```

From the React Native app:

```sh
npm install -D /tmp/siliconsocket-react-native-turbo-debugger-0.1.0.tgz
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

Run `pod install`. The package observes React Native's HTTP handler delegate and forwards every callback to the original delegate. The app's URL session still handles request bodies, caching, redirects and cookies. Capture in this version targets the iOS simulator and requests handled by React Native's built-in HTTP handler.

## View the traffic

Run the desktop from the repository root:

```sh
RNTD_LIVE=1 cargo run --locked
```

Launch the Debug mobile app and make a request. The desktop replaces the waiting session with the connected app's name and shows requests as they arrive. It keeps the last session visible when the app disconnects and accepts one connected app at a time.
