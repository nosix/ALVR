package io.github.alvr.android.lib

abstract class DeviceAdapter {

    @Suppress("unused") // publish to native code
    abstract fun getDeviceSettings(): DeviceSettings

    @Suppress("unused") // publish to native code
    abstract fun getTracking(frameIndex: Long): Tracking

    @Suppress("unused") // publish to native code
    abstract fun onRendered(frameIndex: Long)

    private lateinit var mNativeApi: NativeApi

    fun attach(nativeApi: NativeApi) {
        mNativeApi = nativeApi
        nativeApi.setDeviceAdapter(this)
    }
}