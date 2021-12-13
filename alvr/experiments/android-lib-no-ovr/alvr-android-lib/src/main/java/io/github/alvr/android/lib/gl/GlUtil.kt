package io.github.alvr.android.lib.gl

import android.opengl.GLES32
import java.nio.Buffer
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.nio.ShortBuffer

const val COORDS_3D = 3
const val COORDS_2D = 2

fun loadShader(type: Int, shaderCode: String): Int {
    return GLES32.glCreateShader(type).also { shader ->
        GLES32.glShaderSource(shader, shaderCode)
        GLES32.glCompileShader(shader)
    }
}

fun setupProgram(vertexShader: Int, fragmentShader: Int): Int {
    return GLES32.glCreateProgram().also {
        GLES32.glAttachShader(it, vertexShader)
        GLES32.glAttachShader(it, fragmentShader)
        GLES32.glLinkProgram(it)
    }
}

fun allocateDirectFloatBuffer(array: FloatArray): FloatBuffer {
    return ByteBuffer.allocateDirect(array.size * Float.SIZE_BYTES).run {
        order(ByteOrder.nativeOrder())
        asFloatBuffer().apply {
            put(array)
            position(0)
        }
    }
}

fun allocateDirectShortBuffer(array: ShortArray): ShortBuffer {
    return ByteBuffer.allocateDirect(array.size * Short.SIZE_BYTES).run {
        order(ByteOrder.nativeOrder())
        asShortBuffer().apply {
            put(array)
            position(0)
        }
    }
}

abstract class VertexAttribArray(
    private val name: String,
    private val size: Int,
    private val glType: Int,
    private val byteSize: Int
) {
    fun enable(
        program: Int,
        buffer: Buffer,
        normalized: Boolean = false
    ): Int {
        return GLES32.glGetAttribLocation(program, name).also {
            GLES32.glEnableVertexAttribArray(it)
            GLES32.glVertexAttribPointer(
                it, size, glType, normalized, byteSize * size, buffer
            )
        }
    }
}

class FloatVertexAttribArray(name: String, size: Int) :
    VertexAttribArray(name, size, GLES32.GL_FLOAT, Float.SIZE_BYTES)

fun Int.toErrorString(): String = when (this) {
    GLES32.GL_NO_ERROR -> "no error"
    GLES32.GL_INVALID_ENUM -> "invalid enum"
    GLES32.GL_INVALID_VALUE -> "invalid value"
    GLES32.GL_INVALID_OPERATION -> "invalid operation"
    GLES32.GL_STACK_OVERFLOW -> "stack overflow"
    GLES32.GL_STACK_UNDERFLOW -> "stack underflow"
    GLES32.GL_OUT_OF_MEMORY -> "out of memory"
    else -> "unknown"
}

fun String.throwIfError() {
    GLES32.glGetError().takeIf { it != GLES32.GL_NO_ERROR }?.let {
        throw IllegalStateException("${this}: ${it.toErrorString()} ")
    }
}
