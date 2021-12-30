package io.github.alvr.android.lib

interface ClientEventObserver {
    fun onEventOccurred(eventJson: String)
}