package io.github.alvr.android.lib

import android.util.Log
import io.github.alvr.android.lib.event.ConnectionEvent
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json

class ConnectionObserver(val onEventOccurred: (ConnectionEvent) -> Unit) {

    @Suppress("unused") // publish to native code
    fun onEventOccurred(eventJson: String) {
        Log.d("Observer", eventJson)
        onEventOccurred(Json.decodeFromString<ConnectionEvent>(eventJson))
    }
}