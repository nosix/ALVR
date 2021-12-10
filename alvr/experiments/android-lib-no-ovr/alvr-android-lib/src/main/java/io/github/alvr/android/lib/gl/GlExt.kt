package io.github.alvr.android.lib.gl

import android.opengl.GLES32

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