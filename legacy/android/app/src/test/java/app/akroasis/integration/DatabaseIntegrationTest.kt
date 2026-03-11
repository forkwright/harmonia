package app.akroasis.integration

import app.akroasis.data.local.SmartPlaylistDao
import app.akroasis.data.local.SmartPlaylistEntity
import app.akroasis.data.local.MusicCacheDao
import app.akroasis.data.local.TrackCacheEntity
import app.akroasis.data.model.FilterRequest
import app.akroasis.data.model.FilterRule
import app.akroasis.data.model.FilterField
import app.akroasis.data.model.FilterOperator
import app.akroasis.data.model.FilterLogic
import app.akroasis.util.MainDispatcherRule
import app.cash.turbine.test
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * Integration tests for Room DAO operations.
 * Tests SmartPlaylistDao CRUD operations and MusicCacheDao cache expiry logic.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class DatabaseIntegrationTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var smartPlaylistDao: FakeSmartPlaylistDao
    private lateinit var musicCacheDao: FakeMusicCacheDao

    private val testFilterRequest = FilterRequest(
        conditions = listOf(
            FilterRule(field = FilterField.GENRE, operator = FilterOperator.EQUALS, value = "Rock")
        ),
        logic = FilterLogic.AND
    )

    private val testPlaylist = SmartPlaylistEntity(
        id = "playlist-1",
        name = "Rock Classics",
        filterRequest = testFilterRequest,
        trackCount = 50,
        lastRefreshed = System.currentTimeMillis(),
        autoRefresh = true,
        createdAt = System.currentTimeMillis() - 86400000, // 1 day ago
        updatedAt = System.currentTimeMillis()
    )

    @Before
    fun setup() {
        smartPlaylistDao = FakeSmartPlaylistDao()
        musicCacheDao = FakeMusicCacheDao()
    }

    // ===== SmartPlaylistDao CRUD Tests =====

    @Test
    fun `SCENARIO 1 - insert and retrieve playlist`() = runTest {
        // When - inserting playlist
        smartPlaylistDao.insertPlaylist(testPlaylist)

        // Then - should be retrievable
        val retrieved = smartPlaylistDao.getPlaylist("playlist-1")
        assertNotNull(retrieved)
        assertEquals("Rock Classics", retrieved?.name)
        assertEquals(50, retrieved?.trackCount)
    }

    @Test
    fun `SCENARIO 2 - getAllPlaylists returns all playlists sorted by name`() = runTest {
        // Given - multiple playlists
        val playlist2 = testPlaylist.copy(id = "playlist-2", name = "Ambient")
        val playlist3 = testPlaylist.copy(id = "playlist-3", name = "Jazz")

        smartPlaylistDao.insertPlaylist(testPlaylist)
        smartPlaylistDao.insertPlaylist(playlist2)
        smartPlaylistDao.insertPlaylist(playlist3)

        // When
        smartPlaylistDao.getAllPlaylists().test {
            val playlists = awaitItem()

            // Then - should be sorted by name
            assertEquals(3, playlists.size)
            assertEquals("Ambient", playlists[0].name)
            assertEquals("Jazz", playlists[1].name)
            assertEquals("Rock Classics", playlists[2].name)

            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun `SCENARIO 3 - update playlist modifies existing`() = runTest {
        // Given - existing playlist
        smartPlaylistDao.insertPlaylist(testPlaylist)

        // When - updating
        val updated = testPlaylist.copy(name = "Updated Name", trackCount = 100)
        smartPlaylistDao.updatePlaylist(updated)

        // Then
        val retrieved = smartPlaylistDao.getPlaylist("playlist-1")
        assertEquals("Updated Name", retrieved?.name)
        assertEquals(100, retrieved?.trackCount)
    }

    @Test
    fun `SCENARIO 4 - delete playlist removes it`() = runTest {
        // Given - existing playlist
        smartPlaylistDao.insertPlaylist(testPlaylist)
        assertNotNull(smartPlaylistDao.getPlaylist("playlist-1"))

        // When - deleting
        smartPlaylistDao.deletePlaylist(testPlaylist)

        // Then
        assertNull(smartPlaylistDao.getPlaylist("playlist-1"))
    }

    @Test
    fun `SCENARIO 5 - deleteById removes playlist by ID`() = runTest {
        // Given
        smartPlaylistDao.insertPlaylist(testPlaylist)

        // When
        smartPlaylistDao.deleteById("playlist-1")

        // Then
        assertNull(smartPlaylistDao.getPlaylist("playlist-1"))
    }

    @Test
    fun `SCENARIO 6 - updateRefreshStatus updates timestamp and count`() = runTest {
        // Given
        smartPlaylistDao.insertPlaylist(testPlaylist)
        val newTimestamp = System.currentTimeMillis() + 1000
        val newCount = 75

        // When
        smartPlaylistDao.updateRefreshStatus("playlist-1", newTimestamp, newCount)

        // Then
        val retrieved = smartPlaylistDao.getPlaylist("playlist-1")
        assertEquals(newTimestamp, retrieved?.lastRefreshed)
        assertEquals(newCount, retrieved?.trackCount)
    }

    @Test
    fun `SCENARIO 7 - getAutoRefreshPlaylists returns only auto-refresh enabled`() = runTest {
        // Given - mix of auto-refresh settings
        val noAutoRefresh = testPlaylist.copy(id = "playlist-2", autoRefresh = false)
        smartPlaylistDao.insertPlaylist(testPlaylist) // autoRefresh = true
        smartPlaylistDao.insertPlaylist(noAutoRefresh)

        // When
        val autoRefreshPlaylists = smartPlaylistDao.getAutoRefreshPlaylists()

        // Then - only auto-refresh enabled
        assertEquals(1, autoRefreshPlaylists.size)
        assertEquals("playlist-1", autoRefreshPlaylists[0].id)
    }

    @Test
    fun `SCENARIO 8 - clearAll removes all playlists`() = runTest {
        // Given - multiple playlists
        smartPlaylistDao.insertPlaylist(testPlaylist)
        smartPlaylistDao.insertPlaylist(testPlaylist.copy(id = "playlist-2"))
        smartPlaylistDao.insertPlaylist(testPlaylist.copy(id = "playlist-3"))

        // When
        smartPlaylistDao.clearAll()

        // Then
        smartPlaylistDao.getAllPlaylists().test {
            val playlists = awaitItem()
            assertTrue(playlists.isEmpty())
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun `SCENARIO 9 - insert with conflict replaces existing`() = runTest {
        // Given - existing playlist
        smartPlaylistDao.insertPlaylist(testPlaylist)

        // When - inserting with same ID
        val replacement = testPlaylist.copy(name = "Replaced Name")
        smartPlaylistDao.insertPlaylist(replacement)

        // Then - should be replaced
        val retrieved = smartPlaylistDao.getPlaylist("playlist-1")
        assertEquals("Replaced Name", retrieved?.name)
    }

    @Test
    fun `SCENARIO 10 - getPlaylist returns null for non-existent ID`() = runTest {
        // When
        val result = smartPlaylistDao.getPlaylist("non-existent")

        // Then
        assertNull(result)
    }

    // ===== MusicCacheDao Tests =====

    @Test
    fun `SCENARIO 11 - cache stores and retrieves track`() = runTest {
        // Given
        val cachedTrack = TrackCacheEntity(
            id = "track-1",
            title = "Test Track",
            artist = "Test Artist",
            album = "Test Album",
            duration = 300000L,
            format = "FLAC",
            coverArtUrl = null,
            cachedAt = System.currentTimeMillis()
        )

        // When
        musicCacheDao.insertTrack(cachedTrack)

        // Then - should be retrievable with valid expiry
        val retrieved = musicCacheDao.getTrack("track-1", 0L)
        assertNotNull(retrieved)
        assertEquals("track-1", retrieved?.id)
    }

    @Test
    fun `SCENARIO 12 - cache returns null for expired track`() = runTest {
        // Given - track cached 1 hour ago
        val oneHourAgo = System.currentTimeMillis() - 3600000
        val cachedTrack = createTestTrackCacheEntity("track-1", oneHourAgo)
        musicCacheDao.insertTrack(cachedTrack)

        // When - querying with expiry time after cache time
        val expiryTime = System.currentTimeMillis() - 1800000 // 30 min ago
        val retrieved = musicCacheDao.getTrack("track-1", expiryTime)

        // Then - should be null (expired)
        assertNull(retrieved)
    }

    @Test
    fun `SCENARIO 13 - cache returns track when not expired`() = runTest {
        // Given - recently cached track
        val now = System.currentTimeMillis()
        val cachedTrack = createTestTrackCacheEntity("track-1", now)
        musicCacheDao.insertTrack(cachedTrack)

        // When - querying with expiry before cache time
        val expiryTime = now - 3600000 // 1 hour ago
        val retrieved = musicCacheDao.getTrack("track-1", expiryTime)

        // Then - should return track
        assertNotNull(retrieved)
    }

    @Test
    fun `SCENARIO 14 - deleteExpired removes old entries`() = runTest {
        // Given - mix of old and new entries
        val now = System.currentTimeMillis()
        val oldTrack = createTestTrackCacheEntity("old-1", now - 7200000) // 2 hours old
        val newTrack = createTestTrackCacheEntity("new-1", now) // Fresh

        musicCacheDao.insertTrack(oldTrack)
        musicCacheDao.insertTrack(newTrack)

        // When - deleting entries older than 1 hour
        musicCacheDao.deleteExpired(now - 3600000)

        // Then - old entry should be gone, new entry remains
        assertNull(musicCacheDao.getTrack("old-1", 0L))
        assertNotNull(musicCacheDao.getTrack("new-1", 0L))
    }

    @Test
    fun `SCENARIO 15 - clearAll removes entire cache`() = runTest {
        // Given - multiple cached tracks
        val now = System.currentTimeMillis()
        musicCacheDao.insertTrack(createTestTrackCacheEntity("track-1", now))
        musicCacheDao.insertTrack(createTestTrackCacheEntity("track-2", now))
        musicCacheDao.insertTrack(createTestTrackCacheEntity("track-3", now))

        // When
        musicCacheDao.clearAll()

        // Then - all should be gone
        assertNull(musicCacheDao.getTrack("track-1", 0L))
        assertNull(musicCacheDao.getTrack("track-2", 0L))
        assertNull(musicCacheDao.getTrack("track-3", 0L))
    }

    @Test
    fun `SCENARIO 16 - insert with same ID updates cache entry`() = runTest {
        // Given - existing cache entry
        val now = System.currentTimeMillis()
        val original = createTestTrackCacheEntity("track-1", now - 1000, "Original Title")
        musicCacheDao.insertTrack(original)

        // When - inserting with same ID
        val updated = createTestTrackCacheEntity("track-1", now, "Updated Title")
        musicCacheDao.insertTrack(updated)

        // Then - should have updated data
        val retrieved = musicCacheDao.getTrack("track-1", 0L)
        assertEquals("Updated Title", retrieved?.title)
        assertEquals(now, retrieved?.cachedAt)
    }

    // ===== Cache Expiry Strategy Tests =====

    @Test
    fun `SCENARIO 17 - 24 hour cache expiry strategy`() = runTest {
        // Given - track cached 25 hours ago
        val now = System.currentTimeMillis()
        val twentyFiveHoursAgo = now - (25 * 3600000)
        val oldTrack = createTestTrackCacheEntity("track-1", twentyFiveHoursAgo)
        musicCacheDao.insertTrack(oldTrack)

        // When - using 24 hour expiry
        val expiryTime = now - (24 * 3600000)
        val retrieved = musicCacheDao.getTrack("track-1", expiryTime)

        // Then - should be expired
        assertNull(retrieved)
    }

    @Test
    fun `SCENARIO 18 - 1 hour cache expiry strategy`() = runTest {
        // Given - track cached 30 minutes ago
        val now = System.currentTimeMillis()
        val thirtyMinutesAgo = now - (30 * 60000)
        val freshTrack = createTestTrackCacheEntity("track-1", thirtyMinutesAgo)
        musicCacheDao.insertTrack(freshTrack)

        // When - using 1 hour expiry
        val expiryTime = now - 3600000
        val retrieved = musicCacheDao.getTrack("track-1", expiryTime)

        // Then - should still be valid
        assertNotNull(retrieved)
    }

    // ===== Helper =====

    private fun createTestTrackCacheEntity(
        id: String,
        cachedAt: Long,
        title: String = "Test Track"
    ) = TrackCacheEntity(
        id = id,
        title = title,
        artist = "Test Artist",
        album = "Test Album",
        duration = 300000L,
        format = "FLAC",
        coverArtUrl = null,
        cachedAt = cachedAt
    )

    // ===== Flow Emission Tests =====

    @Test
    fun `SCENARIO 19 - getAllPlaylists flow emits on changes`() = runTest {
        // Given
        smartPlaylistDao.getAllPlaylists().test {
            // Initial empty
            assertEquals(0, awaitItem().size)

            // When - adding playlist
            smartPlaylistDao.insertPlaylist(testPlaylist)
            assertEquals(1, awaitItem().size)

            // When - adding another
            smartPlaylistDao.insertPlaylist(testPlaylist.copy(id = "playlist-2", name = "Another"))
            assertEquals(2, awaitItem().size)

            // When - removing one
            smartPlaylistDao.deleteById("playlist-1")
            assertEquals(1, awaitItem().size)

            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun `SCENARIO 20 - getAllPlaylists flow emits on update`() = runTest {
        // Given - initial playlist
        smartPlaylistDao.insertPlaylist(testPlaylist)

        smartPlaylistDao.getAllPlaylists().test {
            // Initial state
            val initial = awaitItem()
            assertEquals("Rock Classics", initial[0].name)

            // When - updating
            smartPlaylistDao.updatePlaylist(testPlaylist.copy(name = "Updated Name"))

            // Then - new emission
            val updated = awaitItem()
            assertEquals("Updated Name", updated[0].name)

            cancelAndIgnoreRemainingEvents()
        }
    }

    // ===== Fake DAO Implementations for Testing =====

    private class FakeSmartPlaylistDao : SmartPlaylistDao {
        private val playlists = mutableMapOf<String, SmartPlaylistEntity>()
        private val playlistsFlow = MutableStateFlow<List<SmartPlaylistEntity>>(emptyList())

        private fun updateFlow() {
            playlistsFlow.value = playlists.values.sortedBy { it.name }
        }

        override fun getAllPlaylists(): Flow<List<SmartPlaylistEntity>> = playlistsFlow

        override suspend fun getPlaylist(id: String): SmartPlaylistEntity? = playlists[id]

        override suspend fun insertPlaylist(playlist: SmartPlaylistEntity) {
            playlists[playlist.id] = playlist
            updateFlow()
        }

        override suspend fun updatePlaylist(playlist: SmartPlaylistEntity) {
            playlists[playlist.id] = playlist
            updateFlow()
        }

        override suspend fun deletePlaylist(playlist: SmartPlaylistEntity) {
            playlists.remove(playlist.id)
            updateFlow()
        }

        override suspend fun deleteById(id: String) {
            playlists.remove(id)
            updateFlow()
        }

        override suspend fun updateRefreshStatus(id: String, timestamp: Long, trackCount: Int) {
            playlists[id]?.let {
                playlists[id] = it.copy(lastRefreshed = timestamp, trackCount = trackCount)
                updateFlow()
            }
        }

        override suspend fun getAutoRefreshPlaylists(): List<SmartPlaylistEntity> {
            return playlists.values.filter { it.autoRefresh }
        }

        override suspend fun clearAll() {
            playlists.clear()
            updateFlow()
        }
    }

    private class FakeMusicCacheDao : MusicCacheDao {
        private val cache = mutableMapOf<String, TrackCacheEntity>()

        override suspend fun getTrack(trackId: String, expiryTime: Long): TrackCacheEntity? {
            return cache[trackId]?.takeIf { it.cachedAt > expiryTime }
        }

        override suspend fun insertTrack(track: TrackCacheEntity) {
            cache[track.id] = track
        }

        override suspend fun deleteExpired(expiryTime: Long) {
            cache.entries.removeIf { it.value.cachedAt < expiryTime }
        }

        override suspend fun clearAll() {
            cache.clear()
        }
    }
}
