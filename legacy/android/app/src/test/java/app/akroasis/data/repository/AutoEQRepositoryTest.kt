package app.akroasis.data.repository

import app.akroasis.data.model.FilterType
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

class AutoEQRepositoryTest {

    private lateinit var repository: AutoEQRepository

    companion object {
        private const val HD_600_MODEL = "HD 600"
    }

    @Before
    fun setup() {
        repository = AutoEQRepository()
    }

    @Test
    fun `getAvailableProfiles returns 4 profiles`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        assertEquals(4, profiles.size)
    }

    @Test
    fun `getAvailableProfiles includes HD 600`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        val hd600 = profiles.find { it.model == HD_600_MODEL }
        assertNotNull(hd600)
        assertEquals("Sennheiser", hd600!!.manufacturer)
    }

    @Test
    fun `getAvailableProfiles includes HD 650`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        val hd650 = profiles.find { it.model == "HD 650" }
        assertNotNull(hd650)
        assertEquals("Sennheiser", hd650!!.manufacturer)
    }

    @Test
    fun `getAvailableProfiles includes DT 770 Pro`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        val dt770 = profiles.find { it.model == "DT 770 Pro 80 Ohm" }
        assertNotNull(dt770)
        assertEquals("Beyerdynamic", dt770!!.manufacturer)
    }

    @Test
    fun `getAvailableProfiles includes ATH-M50x`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        val athM50x = profiles.find { it.model == "ATH-M50x" }
        assertNotNull(athM50x)
        assertEquals("Audio-Technica", athM50x!!.manufacturer)
    }

    @Test
    fun `HD 600 profile has correct number of bands`() {
        // When
        val profiles = repository.getAvailableProfiles()
        val hd600 = profiles.find { it.model == HD_600_MODEL }

        // Then
        assertNotNull(hd600)
        assertEquals(7, hd600!!.parametricEq.size)
    }

    @Test
    fun `HD 650 profile has correct number of bands`() {
        // When
        val profiles = repository.getAvailableProfiles()
        val hd650 = profiles.find { it.model == "HD 650" }

        // Then
        assertNotNull(hd650)
        assertEquals(6, hd650!!.parametricEq.size)
    }

    @Test
    fun `DT 770 Pro profile has correct number of bands`() {
        // When
        val profiles = repository.getAvailableProfiles()
        val dt770 = profiles.find { it.model == "DT 770 Pro 80 Ohm" }

        // Then
        assertNotNull(dt770)
        assertEquals(7, dt770!!.parametricEq.size)
    }

    @Test
    fun `ATH-M50x profile has correct number of bands`() {
        // When
        val profiles = repository.getAvailableProfiles()
        val athM50x = profiles.find { it.model == "ATH-M50x" }

        // Then
        assertNotNull(athM50x)
        assertEquals(7, athM50x!!.parametricEq.size)
    }

    @Test
    fun `searchProfiles with empty query returns all profiles`() {
        // When
        val results = repository.searchProfiles("")

        // Then
        assertEquals(4, results.size)
    }

    @Test
    fun `searchProfiles with blank query returns all profiles`() {
        // When
        val results = repository.searchProfiles("   ")

        // Then
        assertEquals(4, results.size)
    }

    @Test
    fun `searchProfiles finds Sennheiser profiles by manufacturer`() {
        // When
        val results = repository.searchProfiles("Sennheiser")

        // Then
        assertEquals(2, results.size)
        assertTrue(results.all { it.manufacturer == "Sennheiser" })
    }

    @Test
    fun `searchProfiles finds profiles case-insensitively`() {
        // When
        val results = repository.searchProfiles("sennheiser")

        // Then
        assertEquals(2, results.size)
    }

    @Test
    fun `searchProfiles finds HD 600 by model`() {
        // When
        val results = repository.searchProfiles(HD_600_MODEL)

        // Then
        assertEquals(1, results.size)
        assertEquals(HD_600_MODEL, results[0].model)
    }

    @Test
    fun `searchProfiles finds Beyerdynamic by manufacturer`() {
        // When
        val results = repository.searchProfiles("Beyerdynamic")

        // Then
        assertEquals(1, results.size)
        assertEquals("DT 770 Pro 80 Ohm", results[0].model)
    }

    @Test
    fun `searchProfiles finds Audio-Technica by manufacturer`() {
        // When
        val results = repository.searchProfiles("Audio-Technica")

        // Then
        assertEquals(1, results.size)
        assertEquals("ATH-M50x", results[0].model)
    }

    @Test
    fun `searchProfiles finds by partial model name`() {
        // When
        val results = repository.searchProfiles("M50")

        // Then
        assertEquals(1, results.size)
        assertEquals("ATH-M50x", results[0].model)
    }

    @Test
    fun `searchProfiles finds by partial manufacturer name`() {
        // When
        val results = repository.searchProfiles("Audio")

        // Then
        assertEquals(1, results.size)
        assertEquals("Audio-Technica", results[0].manufacturer)
    }

    @Test
    fun `searchProfiles returns empty for no matches`() {
        // When
        val results = repository.searchProfiles("Sony WH-1000XM4")

        // Then
        assertEquals(0, results.size)
    }

    @Test
    fun `searchProfiles searches in full name`() {
        // When
        val results = repository.searchProfiles("Sennheiser HD")

        // Then
        assertEquals(2, results.size)
    }

    @Test
    fun `getProfileByName returns correct profile`() {
        // When
        val profile = repository.getProfileByName("Sennheiser $HD_600_MODEL")

        // Then
        assertNotNull(profile)
        assertEquals("Sennheiser", profile!!.manufacturer)
        assertEquals(HD_600_MODEL, profile.model)
    }

    @Test
    fun `getProfileByName returns null for non-existent profile`() {
        // When
        val profile = repository.getProfileByName("Sony WH-1000XM4")

        // Then
        assertNull(profile)
    }

    @Test
    fun `getProfileByName is case-sensitive`() {
        // When
        val profile = repository.getProfileByName("sennheiser hd 600")

        // Then
        assertNull(profile)
    }

    @Test
    fun `getProfileByName works for all profiles`() {
        // Given
        val expectedNames = listOf(
            "Sennheiser $HD_600_MODEL",
            "Sennheiser HD 650",
            "Beyerdynamic DT 770 Pro 80 Ohm",
            "Audio-Technica ATH-M50x"
        )

        // When/Then
        expectedNames.forEach { name ->
            val profile = repository.getProfileByName(name)
            assertNotNull("Profile $name should exist", profile)
            assertEquals(name, profile!!.fullName)
        }
    }

    @Test
    fun `all profiles have valid fullName property`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            assertEquals("${profile.manufacturer} ${profile.model}", profile.fullName)
        }
    }

    @Test
    fun `all profiles have unique full names`() {
        // When
        val profiles = repository.getAvailableProfiles()
        val fullNames = profiles.map { it.fullName }

        // Then
        assertEquals(profiles.size, fullNames.toSet().size)
    }

    @Test
    fun `all parametric bands have valid filter types`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            profile.parametricEq.forEach { band ->
                assertTrue(
                    "Profile ${profile.fullName} has unexpected filter type: ${band.type}",
                    band.type in listOf(
                        FilterType.PEAKING,
                        FilterType.LOW_SHELF,
                        FilterType.HIGH_SHELF
                    )
                )
            }
        }
    }

    @Test
    fun `all parametric bands have positive frequencies`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            profile.parametricEq.forEach { band ->
                assertTrue(
                    "Profile ${profile.fullName} has invalid frequency: ${band.frequency}",
                    band.frequency > 0f
                )
            }
        }
    }

    @Test
    fun `all parametric bands have reasonable gain values`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then - AutoEQ typically uses +/- 10dB max
        profiles.forEach { profile ->
            profile.parametricEq.forEach { band ->
                assertTrue(
                    "Profile ${profile.fullName} has excessive gain: ${band.gain}dB",
                    band.gain in -10f..10f
                )
            }
        }
    }

    @Test
    fun `all parametric bands have positive Q values`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            profile.parametricEq.forEach { band ->
                assertTrue(
                    "Profile ${profile.fullName} has invalid Q: ${band.q}",
                    band.q > 0f
                )
            }
        }
    }

    @Test
    fun `all profiles start with low frequency adjustments`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then - first band should be sub-100Hz
        profiles.forEach { profile ->
            assertTrue(
                "Profile ${profile.fullName} doesn't start with low freq band",
                profile.parametricEq.first().frequency < 150f
            )
        }
    }

    @Test
    fun `all profiles end with high shelf`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            assertEquals(
                "Profile ${profile.fullName} should end with HIGH_SHELF",
                FilterType.HIGH_SHELF,
                profile.parametricEq.last().type
            )
        }
    }

    @Test
    fun `all high shelf bands are at 10kHz`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            val highShelf = profile.parametricEq.last()
            assertEquals(10000f, highShelf.frequency)
        }
    }

    @Test
    fun `HD 600 has expected low shelf boost`() {
        // When
        val profile = repository.getProfileByName("Sennheiser $HD_600_MODEL")

        // Then
        assertNotNull(profile)
        val lowShelf = profile!!.parametricEq.find { it.type == FilterType.LOW_SHELF }
        assertNotNull(lowShelf)
        assertTrue(lowShelf!!.gain > 0f) // Should be boosting bass
    }

    @Test
    fun `all profiles have at least one shelf filter`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then
        profiles.forEach { profile ->
            val hasShelf = profile.parametricEq.any {
                it.type == FilterType.LOW_SHELF || it.type == FilterType.HIGH_SHELF
            }
            assertTrue("Profile ${profile.fullName} has no shelf filters", hasShelf)
        }
    }

    @Test
    fun `profiles contain mix of boosts and cuts`() {
        // When
        val profiles = repository.getAvailableProfiles()

        // Then - AutoEQ profiles should have both positive and negative gain
        profiles.forEach { profile ->
            val hasBoost = profile.parametricEq.any { it.gain > 0f }
            val hasCut = profile.parametricEq.any { it.gain < 0f }
            assertTrue(
                "Profile ${profile.fullName} should have both boosts and cuts",
                hasBoost && hasCut
            )
        }
    }
}
