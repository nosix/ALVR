package io.github.alvr.android.lib.gl

import android.graphics.SurfaceTexture
import android.opengl.GLES11Ext
import android.opengl.GLES32

sealed interface SurfaceTextureHolder {
    val target: Int
    val textureId: Int
    val width: Int
    val height: Int
    val surfaceTexture: SurfaceTexture
    val internalTextureId: Int?

    /**
     * Update texture image
     *
     * Must be called in a thread in GlContext
     * @return A result of glGetError()
     */
    fun updateTexImage(): Int
}

class ExternalOESSurfaceTexture(
    override val textureId: Int,
    override val width: Int,
    override val height: Int,
    override val surfaceTexture: SurfaceTexture
) : SurfaceTextureHolder {
    override val target: Int = GLES11Ext.GL_TEXTURE_EXTERNAL_OES

    override val internalTextureId: Int? = null

    override fun updateTexImage(): Int {
        surfaceTexture.updateTexImage()
        return GLES32.GL_NO_ERROR
    }
}

class Texture2DSurfaceTexture(
    override val textureId: Int,
    override val width: Int,
    override val height: Int,
    override val surfaceTexture: SurfaceTexture,
    override val internalTextureId: Int
) : SurfaceTextureHolder {
    override val target: Int = GLES32.GL_TEXTURE_2D

    override fun updateTexImage(): Int {
        surfaceTexture.updateTexImage()
        GLES32.glCopyImageSubData(
            internalTextureId,
            GLES11Ext.GL_TEXTURE_EXTERNAL_OES,
            0, 0, 0, 0,
            textureId,
            target,
            0, 0, 0, 0,
            width, height, 1
        )
        return GLES32.glGetError()
    }
}