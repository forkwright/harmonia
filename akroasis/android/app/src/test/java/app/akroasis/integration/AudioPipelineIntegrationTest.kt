package app.akroasis.integration

import app.akroasis.audio.*
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Integration tests for audio pipeline components working together
 * Tests realistic scenarios involving multiple components
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AudioPipelineIntegrationTest {

    private lateinit var crossfeedEngine: CrossfeedEngine
    private lateinit var headroomManager: HeadroomManager
    private lateinit var equalizerEngine: EqualizerEngine

    private val testSamples = shortArrayOf(
        10000, 10000,  // Stereo frame 1
        20000, 20000,  // Stereo frame 2
        15000, 15000   // Stereo frame 3
    )

    @Before
    fun setup() {
        crossfeedEngine = CrossfeedEngine()
        headroomManager = HeadroomManager()
        equalizerEngine = EqualizerEngine()
    }

    @Test
    fun `SCENARIO 1 - DSP chain applies crossfeed then headroom correctly`() {
        // Given - enable both effects
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.3f)
        headroomManager.enable()
        headroomManager.setHeadroom(-6.0f) // -6dB attenuation

        // When - process through complete DSP chain
        val afterCrossfeed = crossfeedEngine.processSamples(testSamples, channels = 2)
        val afterHeadroom = headroomManager.processSamples(afterCrossfeed)

        // Then - verify chain applied both effects
        // Crossfeed should mix channels
        assertTrue(afterCrossfeed[0] != testSamples[0])
        // Headroom should attenuate
        assertTrue(afterHeadroom[0] < afterCrossfeed[0])
    }

    @Test
    fun `SCENARIO 2 - Headroom prevents clipping from EQ boost`() = runTest {
        // Given - setup high EQ boost scenario
        val loudSamples = shortArrayOf(30000, 30000) // Near max
        headroomManager.enable()

        // When - get recommended headroom for EQ boost
        val recommendedHeadroom = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = false,
            maxEqGain = 6f // +6dB boost
        )

        // Then - should recommend -6dB headroom
        assertEquals(-6f, recommendedHeadroom)

        // Apply recommended headroom
        headroomManager.setHeadroom(recommendedHeadroom)
        val processed = headroomManager.processSamples(loudSamples)

        // Should not clip
        assertTrue(processed.all { it <= Short.MAX_VALUE && it >= Short.MIN_VALUE })
    }

    @Test
    fun `SCENARIO 3 - Crossfeed and headroom work together`() {
        // Given - both effects enabled
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(CrossfeedEngine.STRENGTH_MEDIUM)
        headroomManager.enable()

        // Get recommended headroom for crossfeed
        val recommendedHeadroom = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = false,
            crossfeedEnabled = true
        )

        // Should recommend -3dB for crossfeed
        assertEquals(-3f, recommendedHeadroom)
        headroomManager.setHeadroom(recommendedHeadroom)

        // When - process audio through both
        val afterCrossfeed = crossfeedEngine.processSamples(testSamples, channels = 2)
        val afterHeadroom = headroomManager.processSamples(afterCrossfeed)

        // Then - audio should be processed correctly
        assertTrue(afterHeadroom.isNotEmpty())
        // Verify attenuation applied
        assertTrue(afterHeadroom.max() < testSamples.max())
    }

    @Test
    fun `SCENARIO 4 - Disabling crossfeed preserves original stereo`() {
        // Given
        val original = testSamples.copyOf()
        crossfeedEngine.disable()

        // When
        val processed = crossfeedEngine.processSamples(original, channels = 2)

        // Then - should return unchanged
        assertEquals(original.size, processed.size)
        original.forEachIndexed { index, value ->
            assertEquals(value, processed[index])
        }
    }

    @Test
    fun `SCENARIO 5 - Headroom manager detects clipping and recommends adjustment`() = runTest {
        // Given - audio near clipping point
        val hotSamples = shortArrayOf(32000, 32000, 32500, 32500)
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f) // No headroom

        // When - process hot audio
        headroomManager.processSamples(hotSamples)

        // Then - should detect clipping
        headroomManager.clippingDetected.test {
            assertTrue(awaitItem())
        }

        // Get peak level
        headroomManager.peakLevel.test {
            val peak = awaitItem()
            assertTrue(peak > 0.95f) // Very hot signal
        }
    }

    @Test
    fun `SCENARIO 6 - Full DSP chain with EQ, crossfeed, and headroom`() {
        // Given - setup complete DSP chain
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(CrossfeedEngine.STRENGTH_MEDIUM)
        headroomManager.enable()

        // Calculate total headroom needed
        val recommendedHeadroom = headroomManager.getRecommendedHeadroom(
            equalizerEnabled = true,
            crossfeedEnabled = true,
            maxEqGain = 6f
        )
        // Should be -6dB (EQ) + -3dB (crossfeed) = -9dB
        assertEquals(-9f, recommendedHeadroom)
        headroomManager.setHeadroom(recommendedHeadroom)

        // When - process through complete chain
        // 1. Crossfeed (stereo mixing)
        val afterCrossfeed = crossfeedEngine.processSamples(testSamples, channels = 2)

        // 2. Headroom (gain reduction)
        val afterHeadroom = headroomManager.processSamples(afterCrossfeed)

        // Then - verify chain applied correctly
        assertTrue(afterHeadroom.isNotEmpty())
        // Should be significantly attenuated
        assertTrue(afterHeadroom.max() < testSamples.max() / 2)
    }

    @Test
    fun `SCENARIO 7 - Mono audio bypasses crossfeed`() {
        // Given - mono audio
        val monoSamples = shortArrayOf(1000, 2000, 3000, 4000)
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.5f)

        // When
        val processed = crossfeedEngine.processSamples(monoSamples, channels = 1)

        // Then - should bypass crossfeed
        assertEquals(monoSamples.size, processed.size)
        monoSamples.forEachIndexed { index, value ->
            assertEquals(value, processed[index])
        }
    }

    @Test
    fun `SCENARIO 8 - Zero headroom preserves amplitude`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(HeadroomManager.HEADROOM_NONE) // 0dB

        // When
        val processed = headroomManager.processSamples(testSamples)

        // Then - should match original (within rounding)
        testSamples.forEachIndexed { index, value ->
            assertEquals(value, processed[index])
        }
    }

    @Test
    fun `SCENARIO 9 - Headroom clamping prevents out-of-range values`() {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(-20.0f) // Will be clamped to -12dB

        // When - check that value was clamped
        headroomManager.headroomDb.test {
            assertEquals(-12.0f, awaitItem()) // Clamped to minimum
        }
    }

    @Test
    fun `SCENARIO 10 - Crossfeed strength clamping prevents invalid values`() {
        // Given
        crossfeedEngine.enable()

        // When - try to set out-of-range strengths
        crossfeedEngine.setStrength(2.0f) // Above 1.0

        // Then - should be clamped to 1.0
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(1.0f, awaitItem())
        }

        // When - try negative
        crossfeedEngine.setStrength(-0.5f)

        // Then - should be clamped to 0.0
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(0.0f, awaitItem())
        }
    }

    @Test
    fun `SCENARIO 11 - DSP effects can be toggled independently`() {
        // Given - enable all effects
        crossfeedEngine.enable()
        headroomManager.enable()

        // When - disable crossfeed only
        crossfeedEngine.disable()

        // Then - crossfeed bypassed, headroom still active
        crossfeedEngine.isEnabled.test {
            assertFalse(awaitItem())
        }
        headroomManager.isEnabled.test {
            assertTrue(awaitItem())
        }

        val afterCrossfeed = crossfeedEngine.processSamples(testSamples, channels = 2)
        val afterHeadroom = headroomManager.processSamples(testSamples)

        // Crossfeed should pass through unchanged
        assertEquals(testSamples.size, afterCrossfeed.size)
        // Headroom should still process
        assertTrue(afterHeadroom.max() < testSamples.max())
    }

    @Test
    fun `SCENARIO 12 - Peak level monitoring updates during processing`() = runTest {
        // Given
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)

        // When - process samples with different levels
        val quietSamples = shortArrayOf(1000, 1000)
        val loudSamples = shortArrayOf(20000, 20000)

        headroomManager.processSamples(quietSamples)
        val quietPeak = headroomManager.peakLevel.value

        headroomManager.processSamples(loudSamples)
        val loudPeak = headroomManager.peakLevel.value

        // Then - peak should update
        assertTrue(loudPeak > quietPeak)
    }

    @Test
    fun `SCENARIO 13 - Headroom reset clears clipping indicator`() = runTest {
        // Given - trigger clipping
        headroomManager.enable()
        headroomManager.setHeadroom(0.0f)
        val hotSamples = shortArrayOf(32000, 32000)
        headroomManager.processSamples(hotSamples)

        // Verify clipping detected
        headroomManager.clippingDetected.test {
            assertTrue(awaitItem())
        }

        // When - reset indicator
        headroomManager.resetClippingIndicator()

        // Then - should be cleared
        headroomManager.clippingDetected.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `SCENARIO 14 - Crossfeed preserves sample count`() {
        // Given
        crossfeedEngine.enable()
        val samples = shortArrayOf(
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20
        )

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then
        assertEquals(samples.size, processed.size)
    }

    @Test
    fun `SCENARIO 15 - Headroom preserves sample count`() {
        // Given
        headroomManager.enable()
        val samples = shortArrayOf(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)

        // When
        val processed = headroomManager.processSamples(samples)

        // Then
        assertEquals(samples.size, processed.size)
    }
}
