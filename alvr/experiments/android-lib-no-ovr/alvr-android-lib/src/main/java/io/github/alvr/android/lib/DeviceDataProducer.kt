package io.github.alvr.android.lib

import android.util.Log

abstract class DeviceDataProducer {
    companion object {
        private val TAG = DeviceDataProducer::class.simpleName
    }

    abstract val settings: DeviceSettings

    private lateinit var mNativeApi: NativeApi

    fun attach(nativeApi: NativeApi) {
        mNativeApi = nativeApi
        nativeApi.setDeviceDataProducer(this)
    }

    @Suppress("unused") // publish to native code
    fun request(dataKind: Byte) {
        when (dataKind) {
            1.toByte() -> { mNativeApi.setDeviceSettings(settings) }
            else -> Log.e(TAG, "Unknown data kind")
        }
    }
}