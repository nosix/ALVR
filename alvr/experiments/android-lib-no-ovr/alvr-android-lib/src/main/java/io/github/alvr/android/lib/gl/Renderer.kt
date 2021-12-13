package io.github.alvr.android.lib.gl

import android.opengl.EGL14
import android.opengl.GLES11Ext
import android.opengl.GLES32
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.nio.ShortBuffer

class Renderer(
    private val surface: GlSurface,
    width: Int,
    height: Int
) {
    companion object {
        private const val COORDS_3D = 3
        private const val COORDS_2D = 2

        private val VERTEX_SHADER_CODE = """
            attribute vec4 a_Position;
            attribute vec2 a_TextureCoord;
            varying vec2 v_TextureCoord;
            void main() {
                v_TextureCoord = a_TextureCoord;
                gl_Position = a_Position;
            }
        """.trimIndent()

        private val FRAGMENT_SHADER_CODE = """
            #extension GL_OES_EGL_image_external : enable
            precision mediump float;
            uniform samplerExternalOES u_Texture;
            varying vec2 v_TextureCoord;
            void main() {
                gl_FragColor = texture2D(u_Texture, v_TextureCoord);
            }
        """.trimIndent()
    }

    private val mProperties = surface.setup(width, height)

    fun render(frame: SurfaceHolder) {
        surface.render(frame)
    }

    private fun GlSurface.setup(width: Int, height: Int): Properties = withGlContext {
        GLES32.glViewport(0, 0, width, height)
        GLES32.glClearColor(1f, 0f, 1f, 1f)

        val vertexShader = loadShader(GLES32.GL_VERTEX_SHADER, VERTEX_SHADER_CODE)
        val fragmentShader = loadShader(GLES32.GL_FRAGMENT_SHADER, FRAGMENT_SHADER_CODE)
        val program = setupProgram(vertexShader, fragmentShader)

        val squareCoords = floatArrayOf(
            -0.5f, 0.5f, 0.0f, // top left
            0.5f, 0.5f, 0.0f, // top right
            -0.5f, -0.5f, 0.0f, // bottom left
            0.5f, -0.5f, 0.0f // bottom right
        )
        val drawOrder = shortArrayOf(
            0, 1, 2,
            3, 2, 1
        )
        val textureCoord = floatArrayOf(
            0.0f, 0.0f,
            1.0f, 0.0f,
            0.0f, 1.0f,
            1.0f, 1.0f
        )

        val vertexBuffer = allocateDirectFloatBuffer(squareCoords)
        val drawOrderBuffer = allocateDirectShortBuffer(drawOrder)
        val textureCoordBuffer = allocateDirectFloatBuffer(textureCoord)

        return Properties(
            program,
            COORDS_3D * Float.SIZE_BYTES,
            vertexBuffer,
            drawOrder.size,
            drawOrderBuffer,
            COORDS_2D * Float.SIZE_BYTES,
            textureCoordBuffer
        )
    }

    private fun GlSurface.render(frame: SurfaceHolder) = withGlContext {
        frame.surfaceTexture.updateTexImage()

        GLES32.glClear(GLES32.GL_COLOR_BUFFER_BIT)
        GLES32.glUseProgram(mProperties.program)

        val positionHandler = GLES32.glGetAttribLocation(
            mProperties.program, "a_Position"
        ).also {
            GLES32.glEnableVertexAttribArray(it)
            GLES32.glVertexAttribPointer(
                it,
                COORDS_3D,
                GLES32.GL_FLOAT,
                false,
                mProperties.vertexStride,
                mProperties.vertexBuffer
            )
        }

        val textureCoordHandler = GLES32.glGetAttribLocation(
            mProperties.program, "a_TextureCoord"
        ).also {
            GLES32.glEnableVertexAttribArray(it)
            GLES32.glVertexAttribPointer(
                it,
                COORDS_2D,
                GLES32.GL_FLOAT,
                false,
                mProperties.textureCoordStride,
                mProperties.textureCoordBuffer
            )
        }

        GLES32.glGetUniformLocation(mProperties.program, "u_Texture").also {
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

    private fun loadShader(type: Int, shaderCode: String): Int {
        return GLES32.glCreateShader(type).also { shader ->
            GLES32.glShaderSource(shader, shaderCode)
            GLES32.glCompileShader(shader)
        }
    }

    private fun setupProgram(vertexShader: Int, fragmentShader: Int): Int {
        return GLES32.glCreateProgram().also {
            GLES32.glAttachShader(it, vertexShader)
            GLES32.glAttachShader(it, fragmentShader)
//            GLES32.glBindAttribLocation(it, 0, "a_Position")
//            GLES32.glBindAttribLocation(it, 0, "a_TextureCoord")
            GLES32.glLinkProgram(it)
        }
    }

    private fun allocateDirectFloatBuffer(array: FloatArray): FloatBuffer {
        return ByteBuffer.allocateDirect(array.size * Float.SIZE_BYTES).run {
            order(ByteOrder.nativeOrder())
            asFloatBuffer().apply {
                put(array)
                position(0)
            }
        }
    }

    private fun allocateDirectShortBuffer(array: ShortArray): ShortBuffer {
        return ByteBuffer.allocateDirect(array.size * Short.SIZE_BYTES).run {
            order(ByteOrder.nativeOrder())
            asShortBuffer().apply {
                put(array)
                position(0)
            }
        }
    }

    private class Properties(
        val program: Int,
        val vertexStride: Int,
        val vertexBuffer: FloatBuffer,
        val drawOrderCount: Int,
        val drawOrderBuffer: ShortBuffer,
        val textureCoordStride: Int,
        val textureCoordBuffer: FloatBuffer
    )
}