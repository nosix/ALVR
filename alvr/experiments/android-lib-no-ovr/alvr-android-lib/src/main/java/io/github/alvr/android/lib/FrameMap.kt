package io.github.alvr.android.lib

import android.util.Log
import java.util.concurrent.atomic.AtomicLongArray

class FrameMap {

    companion object {
        private val TAG = FrameMap::class.simpleName
        private const val SIZE = 4096
    }

    private val mMap = AtomicLongArray(SIZE)

    fun put(presentationTimeUs: Long, frameIndex: Long) {
        if (frameIndex == 0L) {
            Log.w(TAG, "0 means no value, ignore if frame_index is 0")
        }
        mMap[toKey(presentationTimeUs)] = frameIndex
    }

    fun remove(presentationTimeUs: Long): Long {
        return mMap.getAndSet(toKey(presentationTimeUs), 0);
    }

    private fun toKey(presentationTimeUs: Long): Int = presentationTimeUs.toInt() and (SIZE - 1)
}