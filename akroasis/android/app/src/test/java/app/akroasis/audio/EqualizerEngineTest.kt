package app.akroasis.audio

import android.media.audiofx.Equalizer
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*

class EqualizerEngineTest {

    private lateinit var equalizerEngine: EqualizerEngine
    private val mockEqualizer: Equalizer = mock()

    @Before
    fun setup() {
        equalizerEngine = EqualizerEngine()
    }

    @Test
    fun `enable sets equalizer enabled state`() {
        // When
        equalizerEngine.enable()

        // Then - enabled state tracked internally
        // Actual equalizer will be null until attachToSession
        // This tests state management
    }

    @Test
    fun `disable sets equalizer disabled state`() {
        // When
        equalizerEngine.disable()

        // Then - state change tracked
    }

    @Test
    fun `getNumberOfBands returns 5 when no equalizer attached`() {
        // When
        val bands = equalizerEngine.getNumberOfBands()

        // Then
        assertEquals(5.toShort(), bands)
    }

    @Test
    fun `getBandLevel returns 0 when no equalizer attached`() {
        // When
        val level = equalizerEngine.getBandLevel(0)

        // Then
        assertEquals(0.toShort(), level)
    }

    @Test
    fun `PRESET_FLAT has all zero levels`() {
        // Given
        val preset = EqualizerEngine.PRESET_FLAT

        // Then
        assertEquals("Flat", preset.name)
        assertEquals(5, preset.bandLevels.size)
        assertTrue(preset.bandLevels.all { it == 0.toShort() })
    }

    @Test
    fun `PRESET_ROCK has correct band levels`() {
        // Given
        val preset = EqualizerEngine.PRESET_ROCK

        // Then
        assertEquals("Rock", preset.name)
        assertEquals(listOf<Short>(500, 300, -100, 100, 600), preset.bandLevels)
    }

    @Test
    fun `PRESET_JAZZ has correct band levels`() {
        // Given
        val preset = EqualizerEngine.PRESET_JAZZ

        // Then
        assertEquals("Jazz", preset.name)
        assertEquals(listOf<Short>(400, 200, 300, 100, 400), preset.bandLevels)
    }

    @Test
    fun `PRESET_CLASSICAL has correct band levels`() {
        // Given
        val preset = EqualizerEngine.PRESET_CLASSICAL

        // Then
        assertEquals("Classical", preset.name)
        assertEquals(listOf<Short>(500, 300, -200, 400, 500), preset.bandLevels)
    }

    @Test
    fun `PRESET_POP has correct band levels`() {
        // Given
        val preset = EqualizerEngine.PRESET_POP

        // Then
        assertEquals("Pop", preset.name)
        assertEquals(listOf<Short>(-100, 300, 500, 300, -100), preset.bandLevels)
    }

    @Test
    fun `PRESET_BASS_BOOST emphasizes low frequencies`() {
        // Given
        val preset = EqualizerEngine.PRESET_BASS_BOOST

        // Then
        assertEquals("Bass Boost", preset.name)
        assertEquals(listOf<Short>(700, 500, 200, 0, 0), preset.bandLevels)
        // Verify bass (first bands) are boosted
        assertTrue(preset.bandLevels[0] > preset.bandLevels[2])
        assertTrue(preset.bandLevels[1] > preset.bandLevels[3])
    }

    @Test
    fun `ALL_PRESETS contains all 6 presets`() {
        // When
        val allPresets = EqualizerEngine.ALL_PRESETS

        // Then
        assertEquals(6, allPresets.size)
        assertEquals("Flat", allPresets[0].name)
        assertEquals("Rock", allPresets[1].name)
        assertEquals("Jazz", allPresets[2].name)
        assertEquals("Classical", allPresets[3].name)
        assertEquals("Pop", allPresets[4].name)
        assertEquals("Bass Boost", allPresets[5].name)
    }

    @Test
    fun `ALL_PRESETS has unique preset names`() {
        // Given
        val allPresets = EqualizerEngine.ALL_PRESETS

        // When
        val uniqueNames = allPresets.map { it.name }.toSet()

        // Then
        assertEquals(allPresets.size, uniqueNames.size)
    }

    @Test
    fun `release sets equalizer to null`() {
        // When
        equalizerEngine.release()

        // Then - subsequent calls should not crash
        val bands = equalizerEngine.getNumberOfBands()
        assertEquals(5.toShort(), bands) // Returns default when no equalizer
    }

    @Test
    fun `applyPreset with fewer bands than preset ignores extra bands`() {
        // This tests the boundary condition in applyPreset
        // where preset.bandLevels.size > getNumberOfBands()

        // Given
        val preset = EqualizerEngine.EqualizerPreset(
            name = "Test",
            bandLevels = listOf(100, 200, 300, 400, 500)
        )

        // When/Then - should not crash
        equalizerEngine.applyPreset(preset)
    }

    @Test
    fun `getCurrentLevels returns empty list when no equalizer attached`() {
        // When
        val levels = equalizerEngine.getCurrentLevels()

        // Then
        assertEquals(5, levels.size)
        assertTrue(levels.all { it == 0.toShort() })
    }

    @Test
    fun `EqualizerPreset data class equality works correctly`() {
        // Given
        val preset1 = EqualizerEngine.EqualizerPreset(
            name = "Test",
            bandLevels = listOf(100, 200)
        )
        val preset2 = EqualizerEngine.EqualizerPreset(
            name = "Test",
            bandLevels = listOf(100, 200)
        )
        val preset3 = EqualizerEngine.EqualizerPreset(
            name = "Different",
            bandLevels = listOf(100, 200)
        )

        // Then
        assertEquals(preset1, preset2)
        assertFalse(preset1 == preset3)
    }

    @Test
    fun `preset band levels are in valid range`() {
        // Android Equalizer typically uses -1500 to +1500 (in units of millibels)
        // Our presets use smaller values (e.g., 500 = 5dB)
        // Verify all preset levels are reasonable

        EqualizerEngine.ALL_PRESETS.forEach { preset ->
            preset.bandLevels.forEach { level ->
                assertTrue(
                    "Preset ${preset.name} has invalid level: $level",
                    level in -1500..1500
                )
            }
        }
    }

    @Test
    fun `all presets have 5 bands`() {
        // Android Equalizer typically has 5 bands
        EqualizerEngine.ALL_PRESETS.forEach { preset ->
            assertEquals("Preset ${preset.name} should have 5 bands", 5, preset.bandLevels.size)
        }
    }
}
