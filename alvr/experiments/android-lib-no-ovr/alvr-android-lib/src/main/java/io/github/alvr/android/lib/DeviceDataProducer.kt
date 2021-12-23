package io.github.alvr.android.lib

abstract class DeviceDataProducer {

    @Suppress("unused") // publish to native code
    abstract val deviceSettings: DeviceSettings

    @Suppress("unused") // publish to native code
    abstract val tracking: Tracking

    private lateinit var mNativeApi: NativeApi

    fun attach(nativeApi: NativeApi) {
        mNativeApi = nativeApi
        nativeApi.setDeviceDataProducer(this)
    }
}