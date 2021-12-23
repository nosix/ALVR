package io.github.alvr.android.app

import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.google.ar.sceneform.math.Quaternion
import com.google.ar.sceneform.math.Vector3
import io.github.alvr.android.lib.DeviceDataProducer
import io.github.alvr.android.lib.DeviceSettings
import io.github.alvr.android.lib.Tracking
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class DeviceDataProducerImpl(
    override val deviceSettings: DeviceSettings
) : DeviceDataProducer(), DefaultLifecycleObserver {
    override val tracking = Tracking().apply {
        ipd = 0.068606f
        battery = 100
        plugged = true
        setEyeFov(52f)
    }

    private var mHeadPoseOrientation = Quaternion()
    private var mHeadPostPosition = Vector3()

    private fun Quaternion.updateTracking() {
        tracking.headPose[0] = x
        tracking.headPose[1] = y
        tracking.headPose[2] = z
        tracking.headPose[3] = w
    }

    private fun Vector3.updateTracking() {
        tracking.headPose[4] = x
        tracking.headPose[5] = y
        tracking.headPose[6] = z
    }

    var moveVelocityMph = 7.0f
    var moveToForward = false
    var moveToBack = false
    var moveToLeft = false
    var moveToRight = false
    var moveToUp = false
    var moveToDown = false

    var rotateDegreePerSec = 30f
    var rotateUp = false
    var rotateDown = false
    var rotateLeft = false
    var rotateRight = false

    private val delayTimeMillis = 16L

    override fun onCreate(owner: LifecycleOwner) {
        owner.lifecycleScope.launch {
            owner.repeatOnLifecycle(Lifecycle.State.RESUMED) {
                while (coroutineContext.isActive) {
                    move()
                    delay(delayTimeMillis)
                }
            }
        }
    }

    private fun move() {
        val degree = rotateDegreePerSec * delayTimeMillis / 1000
        var rotation = Quaternion()
        rotation = rotate(rotation, Vector3.up(), -degree, rotateRight)
        rotation = rotate(rotation, Vector3.up(), degree, rotateLeft)
        rotation = rotate(rotation, Vector3.right(), degree, rotateUp)
        rotation = rotate(rotation, Vector3.right(), -degree, rotateDown)
        mHeadPoseOrientation = Quaternion.multiply(
            mHeadPoseOrientation,
            rotation
        )
        mHeadPoseOrientation.updateTracking()

        var moving = Vector3()
        moving = move(moving, Vector3.forward(), moveToForward)
        moving = move(moving, Vector3.back(), moveToBack)
        moving = move(moving, Vector3.left(), moveToLeft)
        moving = move(moving, Vector3.right(), moveToRight)
        moving = move(moving, Vector3.up(), moveToUp)
        moving = move(moving, Vector3.down(), moveToDown)
        mHeadPostPosition = Vector3.add(
            mHeadPostPosition,
            moving.scaled(moveVelocityMph.mphToMpms() * delayTimeMillis)
        )
        mHeadPostPosition.updateTracking()

    }

    private fun rotate(
        current: Quaternion,
        axis: Vector3,
        degree: Float,
        isActive: Boolean
    ): Quaternion {
        if (!isActive) return current
        return Quaternion.multiply(
            current,
            Quaternion.axisAngle(axis, degree)
        )
    }

    private fun move(current: Vector3, direction: Vector3, isActive: Boolean): Vector3 {
        if (!isActive) return current
        return Vector3.add(
            current,
            Quaternion.rotateVector(mHeadPoseOrientation, direction)
        )
    }

    private fun Float.mphToMpms() = this / (60 * 60 * 1000)
}