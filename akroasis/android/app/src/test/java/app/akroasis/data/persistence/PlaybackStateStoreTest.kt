package app.akroasis.data.persistence

import android.content.Context
import android.content.SharedPreferences
import app.akroasis.data.model.Track
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PlaybackStateStoreTest {

    private lateinit var store: PlaybackStateStore
    private lateinit var mockContext: Context
    private lateinit var mockPrefs: SharedPreferences
    private lateinit var mockEditor: SharedPreferences.Editor

    private val testTrack = Track(
        id = "1",
        title = "Test Song",
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = null,
        duration = 180000,
        filePath = "/music/test.flac",
        trackNumber = 1,
        discNumber = 1,
        year = 2024,
        genre = "Rock",
        coverArtUrl = "https://example.com/cover.jpg",
        sampleRate = 44100,
        bitDepth = 16,
        channels = 2,
        codec = "FLAC",
        bitrate = 1000
    )

    private val testState = PlaybackStateStore.PlaybackState(
        currentTrack = testTrack,
        position = 45000,
        queue = listOf(testTrack),
        currentIndex = 0,
        shuffleEnabled = false,
        repeatMode = "OFF",
        playbackSpeed = 1.0f,
        timestamp = System.currentTimeMillis()
    )

    @Before
    fun setup() {
        mockContext = mock()
        mockPrefs = mock()
        mockEditor = mock()

        whenever(mockContext.getSharedPreferences(any(), any())).thenReturn(mockPrefs)
        whenever(mockPrefs.edit()).thenReturn(mockEditor)
        whenever(mockEditor.putString(any(), any())).thenReturn(mockEditor)
        whenever(mockEditor.putLong(any(), any())).thenReturn(mockEditor)
        whenever(mockEditor.putInt(any(), any())).thenReturn(mockEditor)
        whenever(mockEditor.putBoolean(any(), any())).thenReturn(mockEditor)
        whenever(mockEditor.putFloat(any(), any())).thenReturn(mockEditor)

        store = PlaybackStateStore(mockContext)
    }

    @Test
    fun `saveState stores current track`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putString(eq("current_track"), argThat { contains("Test Song") })
        verify(mockEditor).apply()
    }

    @Test
    fun `saveState stores position`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putLong("position", 45000)
    }

    @Test
    fun `saveState stores current index`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putInt("current_index", 0)
    }

    @Test
    fun `saveState stores shuffle enabled`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putBoolean("shuffle_enabled", false)
    }

    @Test
    fun `saveState stores repeat mode`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putString("repeat_mode", "OFF")
    }

    @Test
    fun `saveState stores playback speed`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putFloat("playback_speed", 1.0f)
    }

    @Test
    fun `saveState stores timestamp`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putLong(eq("timestamp"), any())
    }

    @Test
    fun `saveState stores queue`() {
        // When
        store.saveState(testState)

        // Then
        verify(mockEditor).putString(eq("queue"), argThat { contains("Test Song") })
    }

    @Test
    fun `restoreState returns null when no track saved`() {
        // Given
        whenever(mockPrefs.getString("current_track", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNull(restored)
    }

    @Test
    fun `restoreState returns null when track JSON is invalid`() {
        // Given
        whenever(mockPrefs.getString("current_track", null)).thenReturn("invalid json")

        // When
        val restored = store.restoreState()

        // Then
        assertNull(restored)
    }

    @Test
    fun `restoreState returns state with correct position`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong("position", 0)).thenReturn(45000)
        whenever(mockPrefs.getInt("current_index", 0)).thenReturn(0)
        whenever(mockPrefs.getBoolean("shuffle_enabled", false)).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getLong("timestamp", 0)).thenReturn(123456789)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(45000, restored.position)
    }

    @Test
    fun `restoreState returns state with correct shuffle setting`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean("shuffle_enabled", false)).thenReturn(true)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertTrue(restored.shuffleEnabled)
    }

    @Test
    fun `restoreState returns state with correct repeat mode`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("ALL")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals("ALL", restored.repeatMode)
    }

    @Test
    fun `restoreState returns state with correct playback speed`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.5f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(1.5f, restored.playbackSpeed)
    }

    @Test
    fun `restoreState handles empty queue`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(0, restored.queue.size)
    }

    @Test
    fun `restoreState handles queue with multiple tracks`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        val queueJson = """[{"id":"1","title":"Test1","artist":"Artist1","album":"Album1","duration":180000,"format":"FLAC"},{"id":"2","title":"Test2","artist":"Artist2","album":"Album2","duration":240000,"format":"MP3"}]"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getString("queue", null)).thenReturn(queueJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(2, restored.queue.size)
        assertEquals("Test1", restored.queue[0].title)
        assertEquals("Test2", restored.queue[1].title)
    }

    @Test
    fun `restoreState handles invalid queue JSON gracefully`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getString("queue", null)).thenReturn("invalid json")
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(0, restored.queue.size) // Falls back to empty queue
    }

    @Test
    fun `clearState clears all preferences`() {
        // When
        store.clearState()

        // Then
        verify(mockEditor).clear()
        verify(mockEditor).apply()
    }

    @Test
    fun `saveState handles null current track`() {
        // Given
        val stateWithoutTrack = testState.copy(currentTrack = null)

        // When
        store.saveState(stateWithoutTrack)

        // Then - should not crash, other fields still saved
        verify(mockEditor).putLong("position", 45000)
        verify(mockEditor).apply()
    }

    @Test
    fun `saveState handles empty queue`() {
        // Given
        val stateWithEmptyQueue = testState.copy(queue = emptyList())

        // When
        store.saveState(stateWithEmptyQueue)

        // Then
        verify(mockEditor).putString(eq("queue"), eq("[]"))
    }

    @Test
    fun `saveState handles large queue`() {
        // Given
        val largeTracks = (1..100).map { testTrack.copy(id = it.toString()) }
        val stateWithLargeQueue = testState.copy(queue = largeTracks)

        // When
        store.saveState(stateWithLargeQueue)

        // Then - should not crash
        verify(mockEditor).apply()
    }

    @Test
    fun `restoreState preserves current index`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt("current_index", 0)).thenReturn(5)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals(5, restored.currentIndex)
    }

    @Test
    fun `restoreState defaults to OFF for null repeat mode`() {
        // Given
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn(null)
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)
        whenever(mockPrefs.getString("queue", null)).thenReturn(null)

        // When
        val restored = store.restoreState()

        // Then
        assertNotNull(restored)
        assertEquals("OFF", restored.repeatMode)
    }

    @Test
    fun `restoreState handles queue with malformed track entries`() {
        // Given - queue with one valid and one invalid track
        val trackJson = """{"id":"1","title":"Test","artist":"Artist","album":"Album","duration":180000,"format":"FLAC"}"""
        val queueJson = """[{"id":"1","title":"Test1","artist":"Artist1","album":"Album1","duration":180000,"format":"FLAC"},{invalid json}]"""
        whenever(mockPrefs.getString("current_track", null)).thenReturn(trackJson)
        whenever(mockPrefs.getString("queue", null)).thenReturn(queueJson)
        whenever(mockPrefs.getLong(any(), any())).thenReturn(0)
        whenever(mockPrefs.getInt(any(), any())).thenReturn(0)
        whenever(mockPrefs.getBoolean(any(), any())).thenReturn(false)
        whenever(mockPrefs.getString("repeat_mode", "OFF")).thenReturn("OFF")
        whenever(mockPrefs.getFloat("playback_speed", 1.0f)).thenReturn(1.0f)

        // When
        val restored = store.restoreState()

        // Then - should fallback to empty queue due to JSON parse error
        assertNotNull(restored)
        assertEquals(0, restored.queue.size)
    }

    @Test
    fun `saveState uses correct SharedPreferences name`() {
        // When
        PlaybackStateStore(mockContext)

        // Then
        verify(mockContext).getSharedPreferences("playback_state", Context.MODE_PRIVATE)
    }

    @Test
    fun `PlaybackState data class equality works correctly`() {
        // Given
        val state1 = PlaybackStateStore.PlaybackState(
            currentTrack = testTrack,
            position = 100,
            queue = emptyList(),
            currentIndex = 0,
            shuffleEnabled = false,
            repeatMode = "OFF",
            playbackSpeed = 1.0f,
            timestamp = 123
        )
        val state2 = PlaybackStateStore.PlaybackState(
            currentTrack = testTrack,
            position = 100,
            queue = emptyList(),
            currentIndex = 0,
            shuffleEnabled = false,
            repeatMode = "OFF",
            playbackSpeed = 1.0f,
            timestamp = 123
        )

        // Then
        assertEquals(state1, state2)
    }
}
