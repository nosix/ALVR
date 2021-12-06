package io.github.alvr.android.lib

import io.github.alvr.android.lib.event.AlvrCodec
import io.github.alvr.android.lib.event.ConnectionError
import io.github.alvr.android.lib.event.ConnectionEvent
import io.github.alvr.android.lib.event.ConnectionSettings
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import org.junit.Test

import org.junit.Assert.*

class ConnectionEventDeserializeTest {
    @Test
    fun testInitial() {
        assertEquals(
            ConnectionEvent.Initial,
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Initial"
                }
        """.trimIndent()))
    }

    @Test
    fun testServerFound() {
        assertEquals(
            ConnectionEvent.ServerFound("192.168.1.1"),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "ServerFound",
                  "ipaddr": "192.168.1.1"
                }
        """.trimIndent()))
    }
    @Test
    fun testConnected() {
        assertEquals(
            ConnectionEvent.Connected(
                ConnectionSettings(
                    60.0f,
                    AlvrCodec.H264,
                    realtime = true,
                    darkMode = false,
                    dashboardUrl = "http://192.168.1.1:8082/"
                )
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Connected",
                  "settings": {
                    "fps": 60.0,
                    "codec": { "type": "H264" },
                    "realtime": true,
                    "dark_mode": false,
                    "dashboard_url": "http://192.168.1.1:8082/"
                  }
                }
        """.trimIndent()))
    }

    @Test
    fun testStreamStart() {
        assertEquals(
            ConnectionEvent.StreamStart,
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "StreamStart"
                }
        """.trimIndent()))
    }

    @Test
    fun testServerRestart() {
        assertEquals(
            ConnectionEvent.ServerRestart,
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "ServerRestart"
                }
        """.trimIndent()))
    }

    @Test
    fun testError() {
        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.NetworkUnreachable
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "NetworkUnreachable"
                  }
                }
        """.trimIndent()))

        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.ClientUntrusted
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "ClientUntrusted"
                  }
                }
        """.trimIndent()))

        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.IncompatibleVersions
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "IncompatibleVersions"
                  }
                }
        """.trimIndent()))

        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.TimeoutSetUpStream
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "TimeoutSetUpStream"
                  }
                }
        """.trimIndent()))

        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.ServerDisconnected("any cause")
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "ServerDisconnected",
                    "cause": "any cause"
                  }
                }
        """.trimIndent()))

        assertEquals(
            ConnectionEvent.Error(
                ConnectionError.SystemError("any cause")
            ),
            Json.decodeFromString<ConnectionEvent>("""
                {
                  "type": "Error",
                  "error": {
                    "type": "SystemError",
                    "cause": "any cause"
                  }
                }
        """.trimIndent()))
    }

}