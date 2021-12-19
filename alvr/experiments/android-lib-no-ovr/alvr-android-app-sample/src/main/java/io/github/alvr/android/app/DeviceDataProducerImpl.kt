package io.github.alvr.android.app

import io.github.alvr.android.lib.DeviceDataProducer
import io.github.alvr.android.lib.DeviceSettings

class DeviceDataProducerImpl(
    override val deviceSettings: DeviceSettings
) : DeviceDataProducer()