package com.siliconsocket.rnturbodebugger;

import android.os.SystemClock;
import java.io.DataOutputStream;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import okhttp3.Interceptor;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.Response;
import org.json.JSONException;
import org.json.JSONObject;

/** Attaches the same capture hook to a custom OkHttp or Ktor OkHttp client. */
public final class NetworkCapture {
    private static volatile Sink sink;
    private static final Interceptor INTERCEPTOR = NetworkCapture::intercept;

    private NetworkCapture() {}

    static void initialize(String app, String device) {
        sink = new Sink(app, device);
    }

    public static OkHttpClient.Builder instrument(OkHttpClient.Builder builder) {
        if (!builder.interceptors().contains(INTERCEPTOR)) {
            builder.addInterceptor(INTERCEPTOR);
        }
        return builder;
    }

    private static Response intercept(Interceptor.Chain chain) throws IOException {
        Sink target = sink;
        Request request = chain.request();
        if (target == null) {
            return chain.proceed(request);
        }

        String id = UUID.randomUUID().toString();
        String method = request.method();
        String path = request.url().host() + request.url().encodedPath();
        long started = target.elapsed();
        target.emit("start", id, started, method, path, null, null);
        try {
            Response response = chain.proceed(request);
            target.emit("finish", id, target.elapsed(), null, null, response.code(), "response");
            return response;
        } catch (IOException error) {
            target.emit("finish", id, target.elapsed(), null, null, null,
                chain.call().isCanceled() ? "cancelled" : "failed");
            throw error;
        }
    }

    private static final class Sink {
        private final long origin = SystemClock.elapsedRealtime();
        private final String app;
        private final String device;
        private final ThreadPoolExecutor writer = new ThreadPoolExecutor(
            1, 1, 0, TimeUnit.MILLISECONDS, new ArrayBlockingQueue<>(256),
            runnable -> new Thread(runnable, "rn-turbo-debugger"),
            new ThreadPoolExecutor.DiscardPolicy());
        private Socket socket;
        private DataOutputStream output;
        private long retryAt;

        Sink(String app, String device) {
            this.app = app;
            this.device = device;
        }

        long elapsed() {
            return SystemClock.elapsedRealtime() - origin;
        }

        void emit(String type, String id, long at, String method, String path, Integer status, String outcome) {
            writer.execute(() -> {
                if (elapsed() < retryAt) {
                    return;
                }
                try {
                    if (socket == null) {
                        connect();
                    }
                    write(new JSONObject()
                        .put("type", type).put("id", id).put("atMs", at)
                        .put("method", method)
                        .put("path", path == null ? null : path.substring(0, Math.min(path.length(), 2048)))
                        .put("status", status).put("outcome", outcome));
                } catch (IOException | JSONException error) {
                    close();
                    retryAt = elapsed() + 500L;
                }
            });
        }

        private void connect() throws IOException, JSONException {
            socket = new Socket();
            socket.connect(new InetSocketAddress("127.0.0.1", 7410), 250);
            output = new DataOutputStream(socket.getOutputStream());
            write(new JSONObject().put("type", "hello").put("version", 1)
                .put("app", app).put("device", device).put("platform", "Android"));
        }

        private void write(JSONObject message) throws IOException {
            byte[] bytes = message.toString().getBytes(StandardCharsets.UTF_8);
            output.writeInt(bytes.length);
            output.write(bytes);
            output.flush();
        }

        private void close() {
            if (socket != null) {
                try {
                    socket.close();
                } catch (IOException ignored) {
                    // The next event can open a fresh connection.
                }
            }
            socket = null;
            output = null;
        }
    }
}
