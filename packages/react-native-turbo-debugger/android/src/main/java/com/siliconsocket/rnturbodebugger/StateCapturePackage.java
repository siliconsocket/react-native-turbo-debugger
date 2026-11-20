package com.siliconsocket.rnturbodebugger;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import com.facebook.react.BaseReactPackage;
import com.facebook.react.bridge.NativeModule;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.module.model.ReactModuleInfo;
import com.facebook.react.module.model.ReactModuleInfoProvider;
import com.facebook.react.turbomodule.core.interfaces.TurboModule;
import java.util.Collections;

public final class StateCapturePackage extends BaseReactPackage {
    @Override
    public @Nullable NativeModule getModule(@NonNull String name, @NonNull ReactApplicationContext context) {
        return StateModule.NAME.equals(name) ? new StateModule(context) : null;
    }

    @Override
    public @NonNull ReactModuleInfoProvider getReactModuleInfoProvider() {
        return () -> Collections.singletonMap(StateModule.NAME, new ReactModuleInfo(
            StateModule.NAME, StateModule.class.getName(), false, false, false,
            TurboModule.class.isAssignableFrom(StateModule.class)));
    }
}
