package app.nex.client.camera

import android.content.Context
import android.util.Log
import java.io.File
import java.io.FileOutputStream

class NexCameraManager(private val context: Context) {
    companion object {
        private const val TAG = "NexCameraManager"
    }

    interface PhotoCaptureCallback {
        fun onPhotoCaptured(file: File, bytes: ByteArray)
        fun onError(error: Exception)
    }

    fun capturePhotoToCanonicalIngest(photoBytes: ByteArray, callback: PhotoCaptureCallback) {
        try {
            val outputDir = File(context.filesDir, "captures").apply { mkdirs() }
            val photoFile = File(outputDir, "capture_" + System.currentTimeMillis() + ".jpg")
            FileOutputStream(photoFile).use { it.write(photoBytes) }
            Log.i(TAG, "Physical camera photograph captured: " + photoFile.absolutePath)
            callback.onPhotoCaptured(photoFile, photoBytes)
        } catch (e: Exception) {
            Log.e(TAG, "Camera capture error: " + e.message, e)
            callback.onError(e)
        }
    }
}
