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

    external fun setDeviceAdapter(adapter: DeviceAdapter)
    external fun setConnectionObserver(observer: ConnectionObserver)

    external fun onStart()
    external fun onStop()

    external fun onInputBufferAvailable(buffer: InputBuffer)
    external fun onOutputBufferAvailable(frameIndex: Long)
    external fun onRendered(frameIndex: Long)
}