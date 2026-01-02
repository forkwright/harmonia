package app.akroasis.audio

import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Before
import org.junit.Test
import kotlin.math.pow
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class HeadroomManagerTest {

    private lateinit var headroomManager: HeadroomManager

    @Before
    fun setup() {
        headroomManager = HeadroomManager()
    }

    @Test
    fun `initial state is enabled`() = runTest {
        // Then
        headroomManager.isEnabled.test {
            assertTrue(awaitItem())
        }
    }

    @Test
    fun `initial headroom is -3dB`() = runTest {
        // Then
        headroomManager.headroomDb.test {
            assertEquals(-3.0f, awaitItem())
        }
    }

    @Test
    fun `initial clipping detected is false`() = runTest {
        // Then
        headroomManager.clippingDetected.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `initial peak level is 0`() = runTest {
        // Then
        headroomManager.peakLevel.test {
            assertEquals(0f, awaitItem())
        }
    }

    @Test
    fun `enable sets enabled state to true`() = runTest {
        // Given
        headroomManager.disable()

        // When
        headroomManager.enable()

        // Then
        headroomManager.isEnabled.test {
            assertTrue(awaitItem())
        }
    }

    @Test
    fun `disable sets enabled state to false`() = runTest {
        // When
        headroomManager.disable()

        // Then
        headroomManager.isEnabled.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `setHeadroom updates headroom value`() = runTest {
        // When
        headroomManager.setHeadroom(-6.0f)

        // Then
        headroomManager.headroomDb.test {
            assertEquals(-6.0f, awaitItem())
        }
    }

    @Test
    fun `setHeadroom clamps value to -12dB minimum`() = runTest {
        // When
        headroomManager.setHeadroom(-20.0f)

        // Then
        headroomManager.headroomDb.test {
            assertEquals(-12.0f, awaitItem())
        }
    }

    @Test
    fun `setHeadroom clamps value to 0dB maximum`() = runTest {
        // When
        headroomManager.setHeadroom(5.0f)

        // Then
        headroomManager.headroomDb.test {
            assertEquals(0.0f, awaitItem())
        }
    }

    @Test
    fun `processSamples returns original when disabled`() {
        // Given
        headroomManager.disable()
        val samples = shortArrayOf(1000, 2000, 3000)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then
        assertContentEquals(samples, processed)
    }

    @Test
    fun `processSamples reduces amplitude with negative headroom`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(-6.0f) // -6dB = 0.5x amplitude
        val samples = shortArrayOf(1000)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then - verify attenuation (1000 * 0.5 ≈ 500)
        assertTrue(processed[0] < samples[0])
        assertEquals(501, processed[0].toInt()) // -6dB ≈ 0.501x
    }

    @Test
    fun `processSamples with 0dB headroom preserves amplitude`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val samples = shortArrayOf(1000, -1000)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then
        assertContentEquals(samples, processed)
    }

    @Test
    fun `processSamples updates peak level`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val samples = shortArrayOf(16383) // 50% of max

        // When
        headroomManager.processSamples(samples)

        // Then
        headroomManager.peakLevel.test {
            val peak = awaitItem()
            assertTrue(peak > 0.49f && peak < 0.51f) // ~50%
        }
    }

    @Test
    fun `processSamples detects clipping near max value`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val samples = shortArrayOf(Short.MAX_VALUE)

        // When
        headroomManager.processSamples(samples)

        // Then
        headroomManager.clippingDetected.test {
            assertTrue(awaitItem())
        }
    }

    @Test
    fun `processSamples does not detect clipping for moderate levels`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(-6.0f)
        val samples = shortArrayOf(10000)

        // When
        headroomManager.processSamples(samples)

        // Then
        headroomManager.clippingDetected.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `processSamples prevents actual clipping with clamping`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val samples = shortArrayOf(Short.MAX_VALUE, Short.MIN_VALUE)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then - values should be clamped within valid range
        assertTrue(processed[0] <= Short.MAX_VALUE)
        assertTrue(processed[1] >= Short.MIN_VALUE)
    }

    @Test
    fun `processSamples handles empty array`() {
        // Given
        headroomManager.enable()
        val samples = shortArrayOf()

        // When
        val processed = headroomManager.processSamples(samples)

        // Then
        assertEquals(0, processed.size)
    }

    @Test
    fun `processSamples handles negative samples correctly`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(-6.0f)
        val samples = shortArrayOf(-1000)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then - negative values also attenuated
        assertTrue(processed[0] > samples[0]) // Less negative
        assertEquals(-501, processed[0].toInt())
    }

    @Test
    fun `processSamples does not modify input array`() {
        // Given
        headroomManager.enable()
        val samples = shortArrayOf(1000, 2000)
        val original = samples.copyOf()

        // When
        headroomManager.processSamples(samples)

        // Then
        assertContentEquals(original, samples)
    }

    @Test
    fun `resetClippingIndicator clears clipping state`() = runTest {
        // Given - trigger clipping
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        headroomManager.processSamples(shortArrayOf(Short.MAX_VALUE))

        // When
        headroomManager.resetClippingIndicator()

        // Then
        headroomManager.clippingDetected.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `getRecommendedHeadroom returns 0 with no effects`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = false,
            crossfeedEnabled = false
        )

        // Then
        assertEquals(0f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom accounts for EQ gain`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = false,
            maxEqGain = 6f
        )

        // Then - should recommend -6dB headroom
        assertEquals(-6f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom accounts for crossfeed`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = false,
            crossfeedEnabled = true
        )

        // Then - crossfeed adds -3dB
        assertEquals(-3f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom combines EQ and crossfeed`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = true,
            maxEqGain = 4f
        )

        // Then - -4dB (EQ) + -3dB (crossfeed) = -7dB
        assertEquals(-7f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom clamps to -12dB minimum`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = true,
            maxEqGain = 15f
        )

        // Then - should clamp to -12dB
        assertEquals(-12f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom clamps to 0dB maximum`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = false,
            maxEqGain = -5f // Negative EQ gain
        )

        // Then - should clamp to 0dB
        assertEquals(0f, recommended)
    }

    @Test
    fun `getRecommendedHeadroom ignores EQ gain when EQ disabled`() {
        // When
        val recommended = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = false,
            crossfeedEnabled = false,
            maxEqGain = 10f
        )

        // Then - should be 0dB
        assertEquals(0f, recommended)
    }

    @Test
    fun `HEADROOM_SAFE constant is -6dB`() {
        // Then
        assertEquals(-6.0f, HeadroomManager.HEADROOM_SAFE)
    }

    @Test
    fun `HEADROOM_MODERATE constant is -3dB`() {
        // Then
        assertEquals(-3.0f, HeadroomManager.HEADROOM_MODERATE)
    }

    @Test
    fun `HEADROOM_MINIMAL constant is -1dB`() {
        // Then
        assertEquals(-1.0f, HeadroomManager.HEADROOM_MINIMAL)
    }

    @Test
    fun `HEADROOM_NONE constant is 0dB`() {
        // Then
        assertEquals(0.0f, HeadroomManager.HEADROOM_NONE)
    }

    @Test
    fun `headroom constants are in valid range`() {
        // Then
        assertTrue(HeadroomManager.HEADROOM_SAFE in -12f..0f)
        assertTrue(HeadroomManager.HEADROOM_MODERATE in -12f..0f)
        assertTrue(HeadroomManager.HEADROOM_MINIMAL in -12f..0f)
        assertTrue(HeadroomManager.HEADROOM_NONE in -12f..0f)
    }

    @Test
    fun `headroom constants are ordered correctly`() {
        // Then - more negative = safer
        assertTrue(HeadroomManager.HEADROOM_SAFE < HeadroomManager.HEADROOM_MODERATE)
        assertTrue(HeadroomManager.HEADROOM_MODERATE < HeadroomManager.HEADROOM_MINIMAL)
        assertTrue(HeadroomManager.HEADROOM_MINIMAL < HeadroomManager.HEADROOM_NONE)
    }

    @Test
    fun `processSamples with -12dB headroom significantly reduces amplitude`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(-12.0f) // -12dB ≈ 0.25x amplitude
        val samples = shortArrayOf(10000)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then - verify strong attenuation
        assertTrue(processed[0] < samples[0] / 3) // Less than 1/3 original
    }

    @Test
    fun `consecutive clipping triggers persistent indicator`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)

        // When - process many clipping samples
        repeat(150) {
            headroomManager.processSamples(shortArrayOf(Short.MAX_VALUE))
        }

        // Then
        headroomManager.clippingDetected.test {
            assertTrue(awaitItem())
        }
    }

    @Test
    fun `clipping indicator resets after clean samples`() = runTest {
        // Given - trigger clipping
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        headroomManager.processSamples(shortArrayOf(Short.MAX_VALUE))

        // When - process clean samples
        headroomManager.processSamples(shortArrayOf(1000))

        // Then - clipping should clear
        headroomManager.clippingDetected.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `peak level updates to highest value in buffer`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val samples = shortArrayOf(1000, 20000, 5000) // Peak at 20000

        // When
        headroomManager.processSamples(samples)

        // Then
        headroomManager.peakLevel.test {
            val peak = awaitItem()
            val expected = 20000f / Short.MAX_VALUE
            assertTrue(peak > expected - 0.01f && peak < expected + 0.01f)
        }
    }

    @Test
    fun `dB to linear conversion is accurate`() {
        // Given
        headroomManager.enable()

        // -6dB should be ~0.5x amplitude
        headroomManager.setHeadroom(-6.0f)
        val samples1 = shortArrayOf(1000)
        val processed1 = headroomManager.processSamples(samples1)
        assertTrue(processed1[0] in 490..510) // ~500

        // -20dB should be ~0.1x amplitude
        headroomManager.setHeadroom(-12.0f)
        val samples2 = shortArrayOf(1000)
        val processed2 = headroomManager.processSamples(samples2)
        assertTrue(processed2[0] < 300) // Much quieter
    }
}
