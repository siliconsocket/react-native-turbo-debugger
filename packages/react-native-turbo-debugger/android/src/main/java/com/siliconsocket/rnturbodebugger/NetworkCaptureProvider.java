package com.siliconsocket.rnturbodebugger;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.content.pm.ApplicationInfo;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import com.facebook.react.modules.network.OkHttpClientProvider;

public final class NetworkCaptureProvider extends ContentProvider {
    @Override
    public boolean onCreate() {
        Context context = getContext();
        if (context != null && (context.getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0) {
            Context app = context.getApplicationContext();
            NetworkCapture.initialize(app.getApplicationInfo().loadLabel(app.getPackageManager()).toString(), Build.MODEL);
            OkHttpClientProvider.setOkHttpClientFactory(() ->
                NetworkCapture.instrument(OkHttpClientProvider.createClientBuilder(app)).build());
        }
        return true;
    }

    @Override public Cursor query(Uri uri, String[] columns, String selection, String[] args, String sort) { return null; }
    @Override public String getType(Uri uri) { return null; }
    @Override public Uri insert(Uri uri, ContentValues values) { return null; }
    @Override public int update(Uri uri, ContentValues values, String selection, String[] args) { return 0; }
    @Override public int delete(Uri uri, String selection, String[] args) { return 0; }
}
