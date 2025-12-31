// USB DAC detection and configuration for bit-perfect audio
package app.akroasis.audio

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import androidx.annotation.RequiresApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class UsbDacDetector @Inject constructor(
    private val context: Context
) {

    private val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

    private val _connectedDacs = MutableStateFlow<List<UsbDacInfo>>(emptyList())
    val connectedDacs: StateFlow<List<UsbDacInfo>> = _connectedDacs.asStateFlow()

    private val _preferredDac = MutableStateFlow<UsbDacInfo?>(null)
    val preferredDac: StateFlow<UsbDacInfo?> = _preferredDac.asStateFlow()

    private val usbReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            when (intent?.action) {
                UsbManager.ACTION_USB_DEVICE_ATTACHED -> {
                    scanForUsbDacs()
                }
                UsbManager.ACTION_USB_DEVICE_DETACHED -> {
                    val device: UsbDevice? = intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
                    device?.let { removeDisconnectedDac(it) }
                }
                AudioManager.ACTION_HEADSET_PLUG,
                AudioManager.ACTION_HDMI_AUDIO_PLUG -> {
                    scanForUsbDacs()
                }
            }
        }
    }

    fun startMonitoring() {
        val filter = IntentFilter().apply {
            addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED)
            addAction(UsbManager.ACTION_USB_DEVICE_DETACHED)
            addAction(AudioManager.ACTION_HEADSET_PLUG)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                addAction(AudioManager.ACTION_HDMI_AUDIO_PLUG)
            }
        }
        context.registerReceiver(usbReceiver, filter)
        scanForUsbDacs()
    }

    fun stopMonitoring() {
        context.unregisterReceiver(usbReceiver)
    }

    @RequiresApi(Build.VERSION_CODES.M)
    fun scanForUsbDacs() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) {
            _connectedDacs.value = emptyList()
            return
        }

        val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
        val usbDacs = devices.filter { device ->
            device.type == AudioDeviceInfo.TYPE_USB_DEVICE ||
            device.type == AudioDeviceInfo.TYPE_USB_HEADSET
        }.map { device ->
            UsbDacInfo(
                id = device.id,
                productName = device.productName?.toString() ?: "Unknown USB DAC",
                sampleRates = getSupportedSampleRates(device),
                channelCounts = getSupportedChannelCounts(device),
                encodings = getSupportedEncodings(device),
                type = device.type
            )
        }

        _connectedDacs.value = usbDacs

        if (usbDacs.isNotEmpty() && _preferredDac.value == null) {
            _preferredDac.value = usbDacs.first()
        }
    }

    fun setPreferredDac(dac: UsbDacInfo?) {
        _preferredDac.value = dac
    }

    @RequiresApi(Build.VERSION_CODES.M)
    fun getPreferredAudioDeviceInfo(): AudioDeviceInfo? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) return null

        val preferredDacId = _preferredDac.value?.id ?: return null
        val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
        return devices.firstOrNull { it.id == preferredDacId }
    }

    fun hasBitPerfectSupport(): Boolean {
        return Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE
    }

    @RequiresApi(Build.VERSION_CODES.M)
    private fun getSupportedSampleRates(device: AudioDeviceInfo): List<Int> {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            device.sampleRates?.toList() ?: emptyList()
        } else {
            emptyList()
        }
    }

    @RequiresApi(Build.VERSION_CODES.M)
    private fun getSupportedChannelCounts(device: AudioDeviceInfo): List<Int> {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            device.channelCounts?.toList() ?: emptyList()
        } else {
            emptyList()
        }
    }

    @RequiresApi(Build.VERSION_CODES.M)
    private fun getSupportedEncodings(device: AudioDeviceInfo): List<Int> {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            device.encodings?.toList() ?: emptyList()
        } else {
            emptyList()
        }
    }

    private fun removeDisconnectedDac(device: UsbDevice) {
        _connectedDacs.value = _connectedDacs.value.filter {
            it.productName != device.productName
        }

        if (_preferredDac.value?.productName == device.productName) {
            _preferredDac.value = _connectedDacs.value.firstOrNull()
        }
    }
}

data class UsbDacInfo(
    val id: Int,
    val productName: String,
    val sampleRates: List<Int>,
    val channelCounts: List<Int>,
    val encodings: List<Int>,
    val type: Int
) {
    fun supportsHighRes(): Boolean {
        return sampleRates.any { it >= 96000 }
    }

    fun supportsDsd(): Boolean {
        return encodings.any { it == AudioFormat.ENCODING_DSD }
    }

    fun getMaxSampleRate(): Int {
        return sampleRates.maxOrNull() ?: 48000
    }

    fun formatCapabilities(): String {
        val maxSampleRate = getMaxSampleRate()
        val maxChannels = channelCounts.maxOrNull() ?: 2
        val bitDepth = when {
            encodings.contains(AudioFormat.ENCODING_PCM_32BIT) -> "32-bit"
            encodings.contains(AudioFormat.ENCODING_PCM_24BIT_PACKED) -> "24-bit"
            encodings.contains(AudioFormat.ENCODING_PCM_16BIT) -> "16-bit"
            else -> "Unknown"
        }

        val formatParts = mutableListOf<String>()
        formatParts.add("${maxSampleRate / 1000}kHz")
        formatParts.add(bitDepth)
        formatParts.add("${maxChannels}ch")

        if (supportsDsd()) {
            formatParts.add("DSD")
        }

        return formatParts.joinToString(" / ")
    }
}
