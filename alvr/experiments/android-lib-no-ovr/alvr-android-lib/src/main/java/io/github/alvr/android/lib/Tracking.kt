package io.github.alvr.android.lib

@Suppress("unused") // publish to native code
class Tracking {
    @JvmField var ipd: Float = 0f
    @JvmField var battery: Int = 0
    @JvmField var plugged: Boolean = false
    @JvmField val eyeFov = FloatArray(8)
    @JvmField val headPose = FloatArray(7)

    fun setEyeFov(value: Float) {
        eyeFov.indices.forEach {
            eyeFov[it] = value
        }
    }

    fun setHeadPoseOrientation(x: Float, y: Float, z: Float, w: Float) {
        headPose[0] = x
        headPose[1] = y
        headPose[2] = z
        headPose[3] = w
    }

    fun setHeadPosePosition(x: Float, y: Float, z: Float) {
        headPose[4] = x
        headPose[5] = y
        headPose[6] = z
    }
}
