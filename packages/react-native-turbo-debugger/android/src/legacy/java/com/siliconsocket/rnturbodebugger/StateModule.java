package com.siliconsocket.rnturbodebugger;

import androidx.annotation.NonNull;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;

final class StateModule extends ReactContextBaseJavaModule {
    static final String NAME = "RNTurboDebuggerState";

    StateModule(ReactApplicationContext context) {
        super(context);
    }

    @Override
    public @NonNull String getName() {
        return NAME;
    }

    @ReactMethod
    public void enqueueState(String json) {
        NetworkCapture.enqueueState(json);
    }
}
