package app.nex.client.transport

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log

class NexSocketSyncService : Service() {
    companion object {
        private const val TAG = "NexSocketSyncService"
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val host = intent?.getStringExtra("TARGET_HOST") ?: "127.0.0.1"
        val port = intent?.getIntExtra("TARGET_PORT", 8443) ?: 8443
        Log.i(TAG, "Starting physical SMT synchronization against peer " + host + ":" + port)
        return START_NOT_STICKY
    }
}
