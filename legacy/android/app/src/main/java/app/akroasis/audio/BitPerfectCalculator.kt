// Bit-perfect playback capability calculator
package app.akroasis.audio

import android.media.AudioFormat
import android.media.AudioManager
import android.os.Build
import javax.inject.Inject
import javax.inject.Singleton

data class DacCapabilities(
    val maxSampleRate: Int,
    val maxBitDepth: Int,
    val supportedFormats: Set<Int>
)

@Singleton
class BitPerfectCalculator @Inject constructor(
    private val usbDacDetector: UsbDacDetector,
    private val audioManager: AudioManager
) {

    companion object {
        // Default phone DAC capabilities (conservative)
        private val DEFAULT_DAC_CAPABILITIES = DacCapabilities(
            maxSampleRate = 48000,
            maxBitDepth = 16,
            supportedFormats = setOf(AudioFormat.ENCODING_PCM_16BIT)
        )
    }

    private var cachedCapabilities: DacCapabilities? = null

    /**
     * Detect current DAC capabilities
     * Checks for USB DAC first, falls back to phone DAC
     */
    fun detectDacCapabilities(): DacCapabilities {
        // Check cache first
        cachedCapabilities?.let { return it }

        val capabilities = if (usbDacDetector.connectedDacs.value.isNotEmpty()) {
            detectUsbDacCapabilities()
        } else {
            detectPhoneDacCapabilities()
        }

        cachedCapabilities = capabilities
        return capabilities
    }

    private fun detectUsbDacCapabilities(): DacCapabilities {
        // Query USB audio device properties (Android 23+)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
            val usbDevice = devices.firstOrNull {
                it.type == android.media.AudioDeviceInfo.TYPE_USB_DEVICE ||
                it.type == android.media.AudioDeviceInfo.TYPE_USB_HEADSET
            }

            usbDevice?.let { device ->
                val sampleRates = device.sampleRates ?: intArrayOf(48000)
                val maxSampleRate = sampleRates.maxOrNull() ?: 48000

                val encodings = device.encodings ?: intArrayOf(AudioFormat.ENCODING_PCM_16BIT)
                val maxBitDepth = when {
                    encodings.contains(AudioFormat.ENCODING_PCM_32BIT) -> 32
                    encodings.contains(AudioFormat.ENCODING_PCM_24BIT_PACKED) -> 24
                    else -> 16
                }

                return DacCapabilities(
                    maxSampleRate = maxSampleRate,
                    maxBitDepth = maxBitDepth,
                    supportedFormats = encodings.toSet()
                )
            }
        }

        // Fallback for older Android or if detection fails
        return DacCapabilities(
            maxSampleRate = 96000,  // Conservative USB DAC assumption
            maxBitDepth = 24,
            supportedFormats = setOf(
                AudioFormat.ENCODING_PCM_16BIT,
                AudioFormat.ENCODING_PCM_24BIT_PACKED
            )
        )
    }

    private fun detectPhoneDacCapabilities(): DacCapabilities {
        // Most phones support 48kHz/16-bit max
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val devices = audioManager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
            val phoneDevice = devices.firstOrNull {
                it.type == android.media.AudioDeviceInfo.TYPE_BUILTIN_SPEAKER ||
                it.type == android.media.AudioDeviceInfo.TYPE_BUILTIN_EARPIECE
            }

            phoneDevice?.let { device ->
                val sampleRates = device.sampleRates ?: intArrayOf(48000)
                val maxSampleRate = sampleRates.maxOrNull() ?: 48000

                val encodings = device.encodings ?: intArrayOf(AudioFormat.ENCODING_PCM_16BIT)
                val maxBitDepth = when {
                    encodings.contains(AudioFormat.ENCODING_PCM_32BIT) -> 32
                    encodings.contains(AudioFormat.ENCODING_PCM_24BIT_PACKED) -> 24
                    else -> 16
                }

                return DacCapabilities(
                    maxSampleRate = maxSampleRate,
                    maxBitDepth = maxBitDepth,
                    supportedFormats = encodings.toSet()
                )
            }
        }

        return DEFAULT_DAC_CAPABILITIES
    }

    /**
     * Check if track can be played bit-perfect on current DAC
     */
    fun isBitPerfect(
        sampleRate: Int,
        bitDepth: Int?,
        format: String
    ): Boolean {
        val dac = detectDacCapabilities()

        // Must be lossless format
        if (!isLosslessFormat(format)) {
            return false
        }

        // Sample rate must be <= DAC max
        if (sampleRate > dac.maxSampleRate) {
            return false
        }

        // Bit depth must be <= DAC max (if specified)
        bitDepth?.let {
            if (it > dac.maxBitDepth) {
                return false
            }
        }

        return true
    }

    /**
     * Check if track sample rate matches DAC exactly (no resampling)
     */
    fun requiresResampling(sampleRate: Int): Boolean {
        val dac = detectDacCapabilities()

        // Common native sample rates for DACs
        val nativeSampleRates = setOf(44100, 48000, 88200, 96000, 176400, 192000)

        // If track sample rate matches DAC native rate, no resampling
        return sampleRate !in nativeSampleRates || sampleRate > dac.maxSampleRate
    }

    private fun isLosslessFormat(format: String): Boolean {
        return format.uppercase() in setOf(
            "FLAC", "ALAC", "WAV", "AIFF",
            "APE", "WV", "DSD", "DSF", "DFF"
        )
    }

    /**
     * Invalidate cached capabilities (call when USB DAC connected/disconnected)
     */
    fun invalidateCache() {
        cachedCapabilities = null
    }
}
