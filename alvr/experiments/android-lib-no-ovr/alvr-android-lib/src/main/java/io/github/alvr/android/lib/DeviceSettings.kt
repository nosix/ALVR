package io.github.alvr.android.lib

@Suppress("unused") // publish to native code
class DeviceSettings(
    @JvmField val name: String,
    @JvmField val recommendedEyeWidth: Int,
    @JvmField val recommendedEyeHeight: Int,
    @JvmField val availableRefreshRates: FloatArray,
    @JvmField val preferredRefreshRate: Float
)