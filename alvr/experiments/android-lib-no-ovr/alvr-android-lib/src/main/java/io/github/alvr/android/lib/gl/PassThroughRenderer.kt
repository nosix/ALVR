package io.github.alvr.android.lib.gl

import android.opengl.EGL14
import android.opengl.GLES11Ext
import android.opengl.GLES32
import android.util.Log
import java.nio.FloatBuffer
import java.nio.ShortBuffer

class PassThroughRenderer(
    private val surface: GlSurface,
    width: Int,
    height: Int
) : Renderer {
    companion object {
        private val TAG = PassThroughRenderer::class.simpleName
    }

    private class Properties(
        val program: Int,
        val vertexBuffer: FloatBuffer,
        val drawOrderBuffer: ShortBuffer,
        val drawOrderCount: Int
    )

    private val mProperties = surface.setup(width, height)
    private val mFpsCounter = FpsCounter()

    override fun render(frame: SurfaceHolder) {
        surface.render(frame)
    }

    // TODO refactor
    private fun GlSurface.setup(width: Int, height: Int): Properties = withGlContext {
        Log.i(TAG, "setup ($width, $height)")

        GLES32.glViewport(0, 0, width, height)
        GLES32.glClearColor(1f, 0f, 1f, 1f)

        val vertexShader = loadShader(GLES32.GL_VERTEX_SHADER, VERTEX_SHADER)
        val fragmentShader = loadShader(GLES32.GL_FRAGMENT_SHADER, PASS_THROUGH_FRAGMENT_SHADER)
        val program = setupProgram(vertexShader, fragmentShader)

        val vertex = floatArrayOf(
            -1.0f, -1.0f, // bottom left
            1.0f, -1.0f, // bottom right
            -1.0f, 1.0f, // top left
            1.0f, 1.0f // top right
        )
        val drawOrder = shortArrayOf(
            0, 1, 2,
            3, 2, 1
        )

        val vertexBuffer = allocateDirectFloatBuffer(vertex)
        val drawOrderBuffer = allocateDirectShortBuffer(drawOrder)

        return Properties(
            program,
            vertexBuffer,
            drawOrderBuffer,
            drawOrder.size
        )
    }

    private fun GlSurface.render(frame: SurfaceHolder) = withGlContext {
        frame.surfaceTexture.updateTexImage()

        GLES32.glUseProgram(mProperties.program)

        val vertexHandler = ATTRIB_VERTEX.enable(
            mProperties.program,
            mProperties.vertexBuffer
        )

        GLES32.glGetUniformLocation(mProperties.program, UNIFORM_TEXTURE).also {
            GLES32.glActiveTexture(GLES32.GL_TEXTURE0)
            GLES32.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, frame.textureId)
            GLES32.glUniform1i(it, 0) // TEXTURE0
        }

        // NOTE: Relatively slow processing
        GLES32.glDrawElements(
            GLES32.GL_TRIANGLE_STRIP,
            mProperties.drawOrderCount,
            GLES32.GL_UNSIGNED_SHORT,
            mProperties.drawOrderBuffer
        )

        GLES32.glDisableVertexAttribArray(vertexHandler)

        // NOTE: Relatively slow processing
        EGL14.eglSwapBuffers(display, surface)

        mFpsCounter.logFrame()
    }
}