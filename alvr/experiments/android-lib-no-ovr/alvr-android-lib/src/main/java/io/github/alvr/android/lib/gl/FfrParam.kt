package io.github.alvr.android.lib.gl

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlin.math.ceil

@Serializable
data class FfrParam(
    @SerialName("eye_width")
    private val eyeWidth: Int,
    @SerialName("eye_height")
    private val eyeHeight: Int,
    @SerialName("center_size_x")
    private val centerSizeX: Float,
    @SerialName("center_size_y")
    private val centerSizeY: Float,
    @SerialName("center_shift_x")
    private val centerShiftX: Float,
    @SerialName("center_shift_y")
    private val centerShiftY: Float,
    @SerialName("edge_ratio_x")
    private val edgeRatioX: Float,
    @SerialName("edge_ratio_y")
    private val edgeRatioY: Float
) {
    fun getFragmentShader(): String {
        val targetEyeWidth: Float = eyeWidth.toFloat()
        val targetEyeHeight: Float = eyeHeight.toFloat()

        val edgeSizeX: Float = targetEyeWidth - centerSizeX * targetEyeWidth
        val edgeSizeY: Float = targetEyeHeight - centerSizeY * targetEyeHeight

        val centerSizeXAligned: Float =
            1.0f - ceil(edgeSizeX / (edgeRatioX * 2.0f)) * (edgeRatioX * 2.0f) / targetEyeWidth
        val centerSizeYAligned: Float =
            1.0f - ceil(edgeSizeY / (edgeRatioY * 2.0f)) * (edgeRatioY * 2.0f) / targetEyeHeight

        val edgeSizeXAligned: Float = targetEyeWidth - centerSizeXAligned * targetEyeWidth
        val edgeSizeYAligned: Float = targetEyeHeight - centerSizeYAligned * targetEyeHeight

        val centerShiftXAligned: Float =
            ceil(centerShiftX * edgeSizeXAligned / (edgeRatioX * 2.0f)) * (edgeRatioX * 2.0f) / edgeSizeXAligned
        val centerShiftYAligned: Float =
            ceil(centerShiftY * edgeSizeYAligned / (edgeRatioY * 2.0f)) * (edgeRatioY * 2.0f) / edgeSizeYAligned

        val foveationScaleX: Float =
            centerSizeXAligned + (1.0f - centerSizeXAligned) / edgeRatioX
        val foveationScaleY: Float =
            centerSizeYAligned + (1.0f - centerSizeYAligned) / edgeRatioY

        val optimizedEyeWidth: Float = foveationScaleX * targetEyeWidth
        val optimizedEyeHeight: Float = foveationScaleY * targetEyeHeight

        // round the frame dimensions to a number of pixel multiple of 32 for the encoder
        val optimizedEyeWidthAligned: Int = ceil(optimizedEyeWidth / 32f).toInt() * 32
        val optimizedEyeHeightAligned: Int = ceil(optimizedEyeHeight / 32f).toInt() * 32

        val eyeWidthRatioAligned: Float = optimizedEyeWidth / optimizedEyeWidthAligned
        val eyeHeightRatioAligned: Float = optimizedEyeHeight / optimizedEyeHeightAligned

        return FFR_FRAGMENT_SHADER_COMMON_FORMAT.format(
            eyeWidth, eyeHeight,
            optimizedEyeWidthAligned,
            optimizedEyeHeightAligned,
            eyeWidthRatioAligned,
            eyeHeightRatioAligned,
            centerSizeXAligned,
            centerSizeYAligned,
            centerShiftXAligned,
            centerShiftYAligned,
            edgeRatioX,
            edgeRatioY
        ) + FFR_FRAGMENT_SHADER_DECOMPRESS_AXIS_ALIGNED
    }
}