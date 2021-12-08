package io.github.alvr.android.lib

class NativeApi {

    companion object {
        init {
            System.loadLibrary("alvr_android")
        }
    }

    /**
     * Initialize preference values.
     *
     * Call before onStart.
     *
     * @param preferences set preferences, preferences may change
     * @return true when a preference changed
     */
    external fun initPreferences(preferences: AlvrPreferences): Boolean

    external fun setConnectionObserver(observer: ConnectionObserver)

    external fun onCreate()
    external fun onStart()
    external fun onStop()

    external fun notifyAvailableInputBuffer(buffer: InputBuffer)
    external fun notifyAvailableOutputBuffer(frameIndex: Long)
}