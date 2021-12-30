package io.github.alvr.android.lib

import io.github.alvr.android.lib.event.AlvrCodec
import io.github.alvr.android.lib.event.ConnectionError
import io.github.alvr.android.lib.event.ConnectionEvent
import io.github.alvr.android.lib.event.ConnectionSettings
import io.github.alvr.android.lib.gl.FfrParam
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Test

class ConnectionEventSerializeTest {

    private fun String.trimSpace() = replace("\\s+".toRegex(), "")

    @Test
    fun testInitial() {
        assertEquals(
            """
                {
                  "type": "Initial"
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Initial
            )
        )
    }

    @Test
    fun testServerFound() {
        assertEquals(
            """
                {
                  "type": "ServerFound",
                  "ipaddr": "192.168.1.1"
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.ServerFound("192.168.1.1")
            )
        )
    }

    @Test
    fun testConnectedWithoutFfrParam() {
        assertEquals(
            """
                {
                  "type": "Connected",
                  "settings": {
                    "fps": 60.0,
                    "codec": { "type": "H264" },
                    "realtime": true,
                    "dashboard_url": "http://192.168.1.1:8082/",
                    "ffr_param": null
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Connected(
                    ConnectionSettings(
                        60.0f,
                        AlvrCodec.H264,
                        realtime = true,
                        dashboardUrl = "http://192.168.1.1:8082/",
                        ffrParam = null
                    )
                )
            )
        )
    }

    @Test
    fun testConnected() {
        assertEquals(
            """
                {
                  "type": "Connected",
                  "settings": {
                    "fps": 60.0,
                    "codec": { "type": "H264" },
                    "realtime": true,
                    "dashboard_url": "http://192.168.1.1:8082/",
                    "ffr_param": {
                      "eye_width": 1920,
                      "eye_height": 1080,
                      "center_size_x": 1.0,
                      "center_size_y": 2.0,
                      "center_shift_x": 3.0 ,
                      "center_shift_y": 4.0,
                      "edge_ratio_x": 5.0,
                      "edge_ratio_y": 6.0
                    }
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Connected(
                    ConnectionSettings(
                        60.0f,
                        AlvrCodec.H264,
                        realtime = true,
                        dashboardUrl = "http://192.168.1.1:8082/",
                        ffrParam = FfrParam(
                            1920, 1080,
                            1f, 2f,
                            3f, 4f,
                            5f, 6f
                        )
                    )
                )
            )
        )
    }

    @Test
    fun testStreamStart() {
        assertEquals(
            """
                {
                  "type": "StreamStart"
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.StreamStart
            )
        )
    }

    @Test
    fun testServerRestart() {
        assertEquals(
            """
                {
                  "type": "ServerRestart"
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.ServerRestart
            )
        )
    }

    @Test
    fun testError() {
        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "NetworkUnreachable"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.NetworkUnreachable
                )
            )
        )

        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "ClientUntrusted"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.ClientUntrusted
                )
            )
        )

        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "IncompatibleVersions"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.IncompatibleVersions
                )
            )
        )

        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "TimeoutSetUpStream"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.TimeoutSetUpStream
                )
            )
        )

        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "ServerDisconnected",
                    "cause": "any_cause"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.ServerDisconnected("any_cause")
                )
            )
        )

        assertEquals(
            """
                {
                  "type": "Error",
                  "error": {
                    "type": "SystemError",
                    "cause": "any_cause"
                  }
                }
            """.trimSpace(),
            Json.encodeToString<ConnectionEvent>(
                ConnectionEvent.Error(
                    ConnectionError.SystemError("any_cause")
                )
            )
        )
    }

}