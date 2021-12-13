package io.github.alvr.android.lib.gl

import android.opengl.EGL14
import android.opengl.GLES11Ext
import android.opengl.GLES32
import java.nio.FloatBuffer
import java.nio.ShortBuffer

class Renderer(
    private val surface: GlSurface,
    width: Int,
    height: Int
) {
    companion object {
        private val ATTRIB_POSITION = FloatVertexAttribArray("a_Position", COORDS_3D)
        private val ATTRIB_TEXTURE_COORD = FloatVertexAttribArray("a_TextureCoord", COORDS_2D)
        private const val UNIFORM_TEXTURE = "u_Texture"
    }

    private class Properties(
        val program: Int,
        val vertexBuffer: FloatBuffer,
        val textureCoordBuffer: FloatBuffer,
        val drawOrderBuffer: ShortBuffer,
        val drawOrderCount: Int
    )

    private val mProperties = surface.setup(width, height)

    fun render(frame: SurfaceHolder) {
        surface.render(frame)
    }

    private fun GlSurface.setup(width: Int, height: Int): Properties = withGlContext {
        GLES32.glViewport(0, 0, width, height)
        GLES32.glClearColor(1f, 0f, 1f, 1f)

        val vertexShader = loadShader(GLES32.GL_VERTEX_SHADER, PASS_THROUGH_VERTEX_SHADER)
        val fragmentShader = loadShader(GLES32.GL_FRAGMENT_SHADER, PASS_THROUGH_FRAGMENT_SHADER)
        val program = setupProgram(vertexShader, fragmentShader)

        val squareCoords = floatArrayOf(
            -0.5f, 0.5f, 0.0f, // top left
            0.5f, 0.5f, 0.0f, // top right
            -0.5f, -0.5f, 0.0f, // bottom left
            0.5f, -0.5f, 0.0f // bottom right
        )
        val textureCoord = floatArrayOf(
            0.0f, 0.0f,
            1.0f, 0.0f,
            0.0f, 1.0f,
            1.0f, 1.0f
        )
        val drawOrder = shortArrayOf(
            0, 1, 2,
            3, 2, 1
        )

        val vertexBuffer = allocateDirectFloatBuffer(squareCoords)
        val textureCoordBuffer = allocateDirectFloatBuffer(textureCoord)
        val drawOrderBuffer = allocateDirectShortBuffer(drawOrder)

        return Properties(
            program,
            vertexBuffer,
            textureCoordBuffer,
            drawOrderBuffer,
            drawOrder.size
        )
    }

    private fun GlSurface.render(frame: SurfaceHolder) = withGlContext {
        frame.surfaceTexture.updateTexImage()

        GLES32.glClear(GLES32.GL_COLOR_BUFFER_BIT)
        GLES32.glUseProgram(mProperties.program)

        val positionHandler = ATTRIB_POSITION.enable(
            mProperties.program,
            mProperties.vertexBuffer
        )

        val textureCoordHandler = ATTRIB_TEXTURE_COORD.enable(
            mProperties.program,
            mProperties.textureCoordBuffer
        )

        GLES32.glGetUniformLocation(mProperties.program, UNIFORM_TEXTURE).also {
            GLES32.glActiveTexture(GLES32.GL_TEXTURE0)
            GLES32.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, frame.textureId)
            GLES32.glUniform1i(it, 0)
        }

        GLES32.glDrawElements(
            GLES32.GL_TRIANGLES,
            mProperties.drawOrderCount,
            GLES32.GL_UNSIGNED_SHORT,
            mProperties.drawOrderBuffer
        )

        GLES32.glDisableVertexAttribArray(positionHandler)
        GLES32.glDisableVertexAttribArray(textureCoordHandler)

        GLES32.glFlush()
        EGL14.eglSwapBuffers(display, surface)
    }
}