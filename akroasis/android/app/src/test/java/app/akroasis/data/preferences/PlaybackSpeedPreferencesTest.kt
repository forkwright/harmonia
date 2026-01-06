package app.akroasis.data.preferences

import app.akroasis.data.local.ContentType
import app.akroasis.data.local.PlaybackSpeedDao
import app.akroasis.data.local.PlaybackSpeedRecord
import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*

@OptIn(ExperimentalCoroutinesApi::class)
class PlaybackSpeedPreferencesTest {

    private lateinit var preferences: PlaybackSpeedPreferences
    private lateinit var mockDao: PlaybackSpeedDao

    @Before
    fun setup() {
        mockDao = mock()
        preferences = PlaybackSpeedPreferences(mockDao)
    }

    @Test
    fun `getSpeedForTrack returns default 1_0 when no speeds set`() = runTest {
        // Given
        whenever(mockDao.getSpeed(any())).thenReturn(null)

        // When
        val speed = preferences.getSpeedForTrack("track1", "album1")

        // Then
        assertEquals(1.0f, speed)
    }

    @Test
    fun `getSpeedForTrack returns track speed when set`() = runTest {
        // Given
        val trackSpeed = PlaybackSpeedRecord("track1", 1.5f, ContentType.TRACK)
        whenever(mockDao.getSpeed("track1")).thenReturn(trackSpeed)

        // When
        val speed = preferences.getSpeedForTrack("track1", "album1")

        // Then
        assertEquals(1.5f, speed)
    }

    @Test
    fun `getSpeedForTrack falls back to album speed when track not set`() = runTest {
        // Given
        val albumSpeed = PlaybackSpeedRecord("album1", 1.25f, ContentType.ALBUM)
        whenever(mockDao.getSpeed("track1")).thenReturn(null)
        whenever(mockDao.getSpeed("album1")).thenReturn(albumSpeed)

        // When
        val speed = preferences.getSpeedForTrack("track1", "album1")

        // Then
        assertEquals(1.25f, speed)
    }

    @Test
    fun `getSpeedForTrack prioritizes track over album`() = runTest {
        // Given
        val trackSpeed = PlaybackSpeedRecord("track1", 1.5f, ContentType.TRACK)
        val albumSpeed = PlaybackSpeedRecord("album1", 1.25f, ContentType.ALBUM)
        whenever(mockDao.getSpeed("track1")).thenReturn(trackSpeed)
        whenever(mockDao.getSpeed("album1")).thenReturn(albumSpeed)

        // When
        val speed = preferences.getSpeedForTrack("track1", "album1")

        // Then
        assertEquals(1.5f, speed) // Track speed wins
    }

    @Test
    fun `setSpeedForTrack stores track speed`() = runTest {
        // When
        preferences.setSpeedForTrack("track1", 1.75f)

        // Then
        verify(mockDao).setSpeed(
            PlaybackSpeedRecord(
                contentId = "track1",
                speed = 1.75f,
                contentType = ContentType.TRACK
            )
        )
    }

    @Test
    fun `setSpeedForAlbum stores album speed`() = runTest {
        // When
        preferences.setSpeedForAlbum("album1", 1.25f)

        // Then
        verify(mockDao).setSpeed(
            PlaybackSpeedRecord(
                contentId = "album1",
                speed = 1.25f,
                contentType = ContentType.ALBUM
            )
        )
    }

    @Test
    fun `setSpeedForAudiobook stores audiobook speed`() = runTest {
        // When
        preferences.setSpeedForAudiobook("audiobook1", 1.5f)

        // Then
        verify(mockDao).setSpeed(
            PlaybackSpeedRecord(
                contentId = "audiobook1",
                speed = 1.5f,
                contentType = ContentType.AUDIOBOOK
            )
        )
    }

    @Test
    fun `clearSpeed removes speed setting`() = runTest {
        // When
        preferences.clearSpeed("track1")

        // Then
        verify(mockDao).deleteSpeed("track1")
    }

    @Test
    fun `clearAll removes all speed settings`() = runTest {
        // When
        preferences.clearAll()

        // Then
        verify(mockDao).clearAll()
    }

    @Test
    fun `setSpeedForTrack accepts slow speeds`() = runTest {
        // When
        preferences.setSpeedForTrack("track1", 0.5f)

        // Then
        verify(mockDao).setSpeed(
            argThat { speed == 0.5f }
        )
    }

    @Test
    fun `setSpeedForTrack accepts fast speeds`() = runTest {
        // When
        preferences.setSpeedForTrack("track1", 2.5f)

        // Then
        verify(mockDao).setSpeed(
            argThat { speed == 2.5f }
        )
    }

    @Test
    fun `setSpeedForAlbum accepts normal speed`() = runTest {
        // When
        preferences.setSpeedForAlbum("album1", 1.0f)

        // Then
        verify(mockDao).setSpeed(
            argThat { speed == 1.0f }
        )
    }

    @Test
    fun `multiple tracks can have different speeds`() = runTest {
        // When
        preferences.setSpeedForTrack("track1", 1.5f)
        preferences.setSpeedForTrack("track2", 1.25f)

        // Then
        verify(mockDao).setSpeed(argThat { contentId == "track1" && speed == 1.5f })
        verify(mockDao).setSpeed(argThat { contentId == "track2" && speed == 1.25f })
    }

    @Test
    fun `album speed applies to all tracks in album`() = runTest {
        // Given
        val albumSpeed = PlaybackSpeedRecord("album1", 1.25f, ContentType.ALBUM)
        whenever(mockDao.getSpeed("track1")).thenReturn(null)
        whenever(mockDao.getSpeed("track2")).thenReturn(null)
        whenever(mockDao.getSpeed("album1")).thenReturn(albumSpeed)

        // When
        val speed1 = preferences.getSpeedForTrack("track1", "album1")
        val speed2 = preferences.getSpeedForTrack("track2", "album1")

        // Then
        assertEquals(1.25f, speed1)
        assertEquals(1.25f, speed2)
    }

    @Test
    fun `track speed overrides album speed for specific track`() = runTest {
        // Given
        val trackSpeed = PlaybackSpeedRecord("track1", 2.0f, ContentType.TRACK)
        val albumSpeed = PlaybackSpeedRecord("album1", 1.25f, ContentType.ALBUM)
        whenever(mockDao.getSpeed("track1")).thenReturn(trackSpeed)
        whenever(mockDao.getSpeed("track2")).thenReturn(null)
        whenever(mockDao.getSpeed("album1")).thenReturn(albumSpeed)

        // When
        val speed1 = preferences.getSpeedForTrack("track1", "album1")
        val speed2 = preferences.getSpeedForTrack("track2", "album1")

        // Then
        assertEquals(2.0f, speed1) // Track override
        assertEquals(1.25f, speed2) // Album default
    }

    @Test
    fun `clearSpeed allows fallback to album speed`() = runTest {
        // Given
        val albumSpeed = PlaybackSpeedRecord("album1", 1.25f, ContentType.ALBUM)
        whenever(mockDao.getSpeed("track1")).thenReturn(null)
        whenever(mockDao.getSpeed("album1")).thenReturn(albumSpeed)

        // When
        preferences.clearSpeed("track1")
        val speed = preferences.getSpeedForTrack("track1", "album1")

        // Then
        verify(mockDao).deleteSpeed("track1")
        assertEquals(1.25f, speed)
    }

    @Test
    fun `audiobook speed is independent from track speed`() = runTest {
        // When
        preferences.setSpeedForAudiobook("audiobook1", 1.75f)

        // Then
        verify(mockDao).setSpeed(
            argThat { contentType == ContentType.AUDIOBOOK && speed == 1.75f }
        )
    }

    @Test
    fun `DAO is queried for track first, then album`() = runTest {
        // Given
        whenever(mockDao.getSpeed(any())).thenReturn(null)

        // When
        preferences.getSpeedForTrack("track1", "album1")

        // Then
        val inOrder = inOrder(mockDao)
        inOrder.verify(mockDao).getSpeed("track1")
        inOrder.verify(mockDao).getSpeed("album1")
    }

    @Test
    fun `DAO is not queried for album if track speed exists`() = runTest {
        // Given
        val trackSpeed = PlaybackSpeedRecord("track1", 1.5f, ContentType.TRACK)
        whenever(mockDao.getSpeed("track1")).thenReturn(trackSpeed)

        // When
        preferences.getSpeedForTrack("track1", "album1")

        // Then
        verify(mockDao).getSpeed("track1")
        verify(mockDao, never()).getSpeed("album1")
    }
}
