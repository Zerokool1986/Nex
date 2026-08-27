package app.nex.client

import android.app.Application
import android.util.Log

class NexClientApp : Application() {
    companion object {
        private const val TAG = "NexClientApp"
        init {
            try {
                System.loadLibrary("nex_core")
                Log.i(TAG, "libnex_core.so loaded successfully via JNI")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native libnex_core: " + e.message)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "NEX Sovereign Client initialized on Android host")
    }
}
