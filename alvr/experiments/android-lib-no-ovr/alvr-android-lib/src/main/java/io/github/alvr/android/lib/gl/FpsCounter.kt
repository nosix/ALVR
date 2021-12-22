package io.github.alvr.android.lib.gl

import android.util.Log

class FpsCounter {
    companion object {
        private val TAG = FpsCounter::class.simpleName
    }

    private var mStartTime = System.nanoTime()
    private var mFrames = 0

    fun logFrame() {
        mFrames++
        if (System.nanoTime() - mStartTime >= 1_000_000_000) {
            Log.d(TAG, "fps: $mFrames")
            mStartTime = System.nanoTime()
            mFrames = 0
        }
    }
}