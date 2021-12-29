package io.github.alvr.android.lib

import android.view.Surface

class Screen(
    val surface: Surface,
    val width: Int,
    val height: Int,
    val onDetached: () -> Unit
)