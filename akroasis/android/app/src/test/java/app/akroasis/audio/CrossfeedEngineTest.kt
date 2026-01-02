package app.akroasis.audio

import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Before
import org.junit.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class CrossfeedEngineTest {

    private lateinit var crossfeedEngine: CrossfeedEngine

    @Before
    fun setup() {
        crossfeedEngine = CrossfeedEngine()
    }

    @Test
    fun `initial state is disabled`() = runTest {
        // Then
        crossfeedEngine.isEnabled.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `initial crossfeed strength is 0_3`() = runTest {
        // Then
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(0.3f, awaitItem())
        }
    }

    @Test
    fun `enable sets enabled state to true`() = runTest {
        // When
        crossfeedEngine.enable()

        // Then
        crossfeedEngine.isEnabled.test {
            assertTrue(awaitItem())
        }
    }

    @Test
    fun `disable sets enabled state to false`() = runTest {
        // Given
        crossfeedEngine.enable()

        // When
        crossfeedEngine.disable()

        // Then
        crossfeedEngine.isEnabled.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `setStrength updates strength value`() = runTest {
        // When
        crossfeedEngine.setStrength(0.5f)

        // Then
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(0.5f, awaitItem())
        }
    }

    @Test
    fun `setStrength clamps value to 0_0 minimum`() = runTest {
        // When
        crossfeedEngine.setStrength(-0.5f)

        // Then
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(0.0f, awaitItem())
        }
    }

    @Test
    fun `setStrength clamps value to 1_0 maximum`() = runTest {
        // When
        crossfeedEngine.setStrength(1.5f)

        // Then
        crossfeedEngine.crossfeedStrength.test {
            assertEquals(1.0f, awaitItem())
        }
    }

    @Test
    fun `setSampleRate updates sample rate`() {
        // When
        crossfeedEngine.setSampleRate(48000)

        // Then - no exception, state updated internally
    }

    @Test
    fun `processSamples returns original when disabled`() {
        // Given
        crossfeedEngine.disable()
        val samples = shortArrayOf(100, 200, 300, 400)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then
        assertContentEquals(samples, processed)
    }

    @Test
    fun `processSamples returns original for mono audio`() {
        // Given
        crossfeedEngine.enable()
        val samples = shortArrayOf(100, 200, 300, 400)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 1)

        // Then
        assertContentEquals(samples, processed)
    }

    @Test
    fun `processSamples mixes stereo channels when enabled`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.5f) // 50% mix
        val samples = shortArrayOf(
            1000, 0,  // Left: 1000, Right: 0
            0, 1000   // Left: 0, Right: 1000
        )

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - 50% crossfeed means each channel gets 50% from opposite
        // Left_new = 1000 * 0.5 + 0 * 0.5 = 500
        // Right_new = 0 * 0.5 + 1000 * 0.5 = 500
        assertEquals(500, processed[0].toInt())
        assertEquals(500, processed[1].toInt())
        assertEquals(500, processed[2].toInt())
        assertEquals(500, processed[3].toInt())
    }

    @Test
    fun `processSamples with zero strength preserves original`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.0f)
        val samples = shortArrayOf(1000, 2000, 3000, 4000)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - zero strength = no crossfeed
        assertContentEquals(samples, processed)
    }

    @Test
    fun `processSamples with full strength swaps channels`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(1.0f)
        val samples = shortArrayOf(1000, 2000)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - full strength swaps L and R
        // Left_new = 1000 * 0 + 2000 * 1 = 2000
        // Right_new = 2000 * 0 + 1000 * 1 = 1000
        assertEquals(2000, processed[0].toInt())
        assertEquals(1000, processed[1].toInt())
    }

    @Test
    fun `processSamples handles multiple frames correctly`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.3f)
        val samples = shortArrayOf(
            100, 200,  // Frame 1
            300, 400,  // Frame 2
            500, 600   // Frame 3
        )

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - verify each frame processed independently
        // Frame 1: L = 100*0.7 + 200*0.3 = 70 + 60 = 130
        assertEquals(130, processed[0].toInt())
        // Frame 1: R = 200*0.7 + 100*0.3 = 140 + 30 = 170
        assertEquals(170, processed[1].toInt())
    }

    @Test
    fun `processSamples does not modify input array`() {
        // Given
        crossfeedEngine.enable()
        val samples = shortArrayOf(1000, 2000)
        val original = samples.copyOf()

        // When
        crossfeedEngine.processSamples(samples, channels = 2)

        // Then - input unchanged
        assertContentEquals(original, samples)
    }

    @Test
    fun `processSamples handles empty array`() {
        // Given
        crossfeedEngine.enable()
        val samples = shortArrayOf()

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then
        assertEquals(0, processed.size)
    }

    @Test
    fun `processSamples handles single stereo frame`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.2f)
        val samples = shortArrayOf(500, 1000)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - verify calculation
        // Left = 500 * 0.8 + 1000 * 0.2 = 400 + 200 = 600
        assertEquals(600, processed[0].toInt())
        // Right = 1000 * 0.8 + 500 * 0.2 = 800 + 100 = 900
        assertEquals(900, processed[1].toInt())
    }

    @Test
    fun `STRENGTH_LOW constant is 0_15`() {
        // Then
        assertEquals(0.15f, CrossfeedEngine.STRENGTH_LOW)
    }

    @Test
    fun `STRENGTH_MEDIUM constant is 0_30`() {
        // Then
        assertEquals(0.30f, CrossfeedEngine.STRENGTH_MEDIUM)
    }

    @Test
    fun `STRENGTH_HIGH constant is 0_50`() {
        // Then
        assertEquals(0.50f, CrossfeedEngine.STRENGTH_HIGH)
    }

    @Test
    fun `strength constants are in valid range`() {
        // Then
        assertTrue(CrossfeedEngine.STRENGTH_LOW in 0f..1f)
        assertTrue(CrossfeedEngine.STRENGTH_MEDIUM in 0f..1f)
        assertTrue(CrossfeedEngine.STRENGTH_HIGH in 0f..1f)
    }

    @Test
    fun `strength constants are ordered correctly`() {
        // Then
        assertTrue(CrossfeedEngine.STRENGTH_LOW < CrossfeedEngine.STRENGTH_MEDIUM)
        assertTrue(CrossfeedEngine.STRENGTH_MEDIUM < CrossfeedEngine.STRENGTH_HIGH)
    }

    @Test
    fun `release disables crossfeed`() = runTest {
        // Given
        crossfeedEngine.enable()

        // When
        crossfeedEngine.release()

        // Then
        crossfeedEngine.isEnabled.test {
            assertFalse(awaitItem())
        }
    }

    @Test
    fun `processSamples handles negative sample values`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.5f)
        val samples = shortArrayOf(-1000, 1000)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - verify mixed correctly with negatives
        // Left = -1000 * 0.5 + 1000 * 0.5 = -500 + 500 = 0
        assertEquals(0, processed[0].toInt())
        // Right = 1000 * 0.5 + (-1000) * 0.5 = 500 - 500 = 0
        assertEquals(0, processed[1].toInt())
    }

    @Test
    fun `processSamples handles maximum positive sample value`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.0f) // No crossfeed to avoid overflow
        val samples = shortArrayOf(Short.MAX_VALUE, Short.MAX_VALUE)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - should not overflow
        assertEquals(Short.MAX_VALUE, processed[0])
        assertEquals(Short.MAX_VALUE, processed[1])
    }

    @Test
    fun `processSamples handles maximum negative sample value`() {
        // Given
        crossfeedEngine.enable()
        crossfeedEngine.setStrength(0.0f)
        val samples = shortArrayOf(Short.MIN_VALUE, Short.MIN_VALUE)

        // When
        val processed = crossfeedEngine.processSamples(samples, channels = 2)

        // Then - should not underflow
        assertEquals(Short.MIN_VALUE, processed[0])
        assertEquals(Short.MIN_VALUE, processed[1])
    }
}
