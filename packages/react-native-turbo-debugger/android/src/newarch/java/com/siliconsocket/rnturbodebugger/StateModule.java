package com.siliconsocket.rnturbodebugger;

import com.facebook.react.bridge.ReactApplicationContext;

final class StateModule extends NativeRNTurboDebuggerStateSpec {
    StateModule(ReactApplicationContext context) {
        super(context);
    }

    @Override
    public void enqueueState(String json) {
        NetworkCapture.enqueueState(json);
    }
}
