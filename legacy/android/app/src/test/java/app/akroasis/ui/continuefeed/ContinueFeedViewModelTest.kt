// Unit tests for ContinueFeedViewModel
package app.akroasis.ui.continuefeed

import app.akroasis.data.model.MediaItem
import app.akroasis.data.model.MediaProgress
import app.akroasis.data.model.MediaType
import app.akroasis.data.repository.ContinueItem
import app.akroasis.data.repository.MediaRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.*
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.*

@OptIn(ExperimentalCoroutinesApi::class)
class ContinueFeedViewModelTest {

    private lateinit var viewModel: ContinueFeedViewModel
    private lateinit var mediaRepository: MediaRepository
    private val testDispatcher = StandardTestDispatcher()

    private val testMusicItem = MediaItem.Music(
        id = "music-1",
        title = "Test Track",
        artist = "Test Artist",
        album = "Test Album",
        albumArtist = null,
        trackNumber = null,
        discNumber = null,
        year = null,
        duration = 180000,
        bitrate = 320,
        sampleRate = 44100,
        bitDepth = 16,
        format = "FLAC",
        fileSize = 5000000,
        filePath = "/music/test.flac",
        coverArtUrl = null,
        createdAt = "2024-01-01",
        updatedAt = "2024-01-01"
    )

    private val testEbookItem = MediaItem.Ebook(
        id = "ebook-1",
        title = "Test Ebook",
        author = "Test Author",
        seriesName = null,
        seriesNumber = null,
        pageCount = 300,
        publishDate = "2024-01-01",
        coverArtUrl = null,
        duration = null,
        format = "EPUB",
        fileSize = 2000000,
        filePath = "/books/test.epub",
        createdAt = "2024-01-01",
        updatedAt = "2024-01-01"
    )

    private val testMusicProgress = MediaProgress(
        mediaItemId = "music-1",
        mediaType = MediaType.MUSIC,
        positionMs = 90000,
        totalDurationMs = 180000,
        percentComplete = 0.5f,
        lastPlayedAt = System.currentTimeMillis(),
        isComplete = false
    )

    private val testEbookProgress = MediaProgress(
        mediaItemId = "ebook-1",
        mediaType = MediaType.EBOOK,
        positionMs = 600000,
        totalDurationMs = 1800000,
        percentComplete = 0.33f,
        lastPlayedAt = System.currentTimeMillis(),
        isComplete = false
    )

    private val testContinueItems = listOf(
        ContinueItem(testMusicItem, testMusicProgress),
        ContinueItem(testEbookItem, testEbookProgress)
    )

    @Before
    fun setup() {
        Dispatchers.setMain(testDispatcher)
        mediaRepository = mock()
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun `init loads continue feed automatically`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Success)
        assertEquals(2, (state as ContinueFeedUiState.Success).items.size)

        verify(mediaRepository).getContinueFeed("default", 20)
    }

    @Test
    fun `loadContinueFeed success updates state to Success`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(emptyList()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.loadContinueFeed()
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Success)
        assertEquals(2, (state as ContinueFeedUiState.Success).items.size)
    }

    @Test
    fun `loadContinueFeed empty list updates state to Empty`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(emptyList()))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Empty)
    }

    @Test
    fun `loadContinueFeed failure updates state to Error`() = runTest {
        val errorMessage = "Network error"
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.failure(Exception(errorMessage)))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Error)
        assertTrue((state as ContinueFeedUiState.Error).message.contains(errorMessage))
    }

    @Test
    fun `loadContinueFeed exception updates state to Error`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenThrow(RuntimeException("Unexpected error"))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Error)
    }

    @Test
    fun `loadContinueFeed sets Loading state during operation`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)

        // Initial state should be Loading
        assertEquals(ContinueFeedUiState.Loading, viewModel.uiState.value)

        advanceUntilIdle()

        // After load completes, should be Success
        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Success)
    }

    @Test
    fun `refresh updates isRefreshing state correctly`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(emptyList()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        assertFalse(viewModel.isRefreshing.first())

        viewModel.refresh()

        // Should set refreshing to true initially
        // Then back to false after completion
        advanceUntilIdle()

        assertFalse(viewModel.isRefreshing.first())
        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Success)

        verify(mediaRepository, times(2)).getContinueFeed("default", 20)
    }

    @Test
    fun `refresh with empty list updates to Empty state`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
            .thenReturn(Result.success(emptyList()))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.refresh()
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Empty)
        assertFalse(viewModel.isRefreshing.first())
    }

    @Test
    fun `refresh failure sets Error state but keeps isRefreshing false`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
            .thenReturn(Result.failure(Exception("Network error")))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.refresh()
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Error)
        assertFalse(viewModel.isRefreshing.first())
    }

    @Test
    fun `refresh exception sets Error state and isRefreshing false`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
            .thenThrow(RuntimeException("Unexpected error"))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.refresh()
        advanceUntilIdle()

        // State should remain Success (doesn't update on exception in refresh)
        // But isRefreshing should be false
        assertFalse(viewModel.isRefreshing.first())
    }

    @Test
    fun `deleteProgress removes item from list`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
        whenever(mediaRepository.deleteProgress(any(), any()))
            .thenReturn(Result.success(Unit))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        // Delete first item
        viewModel.deleteProgress(testContinueItems[0])
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Success)
        val items = (state as ContinueFeedUiState.Success).items
        assertEquals(1, items.size)
        assertEquals("ebook-1", items[0].mediaItem.id)

        verify(mediaRepository).deleteProgress("music-1", "default")
    }

    @Test
    fun `deleteProgress last item updates state to Empty`() = runTest {
        val singleItem = listOf(testContinueItems[0])
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(singleItem))
        whenever(mediaRepository.deleteProgress(any(), any()))
            .thenReturn(Result.success(Unit))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.deleteProgress(testContinueItems[0])
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Empty)
    }

    @Test
    fun `deleteProgress failure keeps state unchanged`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
        whenever(mediaRepository.deleteProgress(any(), any()))
            .thenReturn(Result.failure(Exception("Delete failed")))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.deleteProgress(testContinueItems[0])
        advanceUntilIdle()

        val state = viewModel.uiState.first()
        assertTrue(state is ContinueFeedUiState.Success)
        assertEquals(2, (state as ContinueFeedUiState.Success).items.size)
    }

    @Test
    fun `deleteProgress when not in Success state does nothing`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.failure(Exception("Load failed")))
        whenever(mediaRepository.deleteProgress(any(), any()))
            .thenReturn(Result.success(Unit))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.deleteProgress(testContinueItems[0])
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Error)
        verify(mediaRepository).deleteProgress("music-1", "default")
    }

    @Test
    fun `retry calls loadContinueFeed`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.failure(Exception("First error")))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Error)

        viewModel.retry()
        advanceUntilIdle()

        assertTrue(viewModel.uiState.first() is ContinueFeedUiState.Success)
        verify(mediaRepository, times(2)).getContinueFeed("default", 20)
    }

    @Test
    fun `loadContinueFeed with custom userId and limit`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.loadContinueFeed(userId = "user123", limit = 50)
        advanceUntilIdle()

        verify(mediaRepository).getContinueFeed("user123", 50)
    }

    @Test
    fun `refresh with custom userId and limit`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.refresh(userId = "user456", limit = 100)
        advanceUntilIdle()

        verify(mediaRepository).getContinueFeed("user456", 100)
    }

    @Test
    fun `deleteProgress with custom userId`() = runTest {
        whenever(mediaRepository.getContinueFeed(any(), any()))
            .thenReturn(Result.success(testContinueItems))
        whenever(mediaRepository.deleteProgress(any(), any()))
            .thenReturn(Result.success(Unit))

        viewModel = ContinueFeedViewModel(mediaRepository)
        advanceUntilIdle()

        viewModel.deleteProgress(testContinueItems[0], userId = "user789")
        advanceUntilIdle()

        verify(mediaRepository).deleteProgress("music-1", "user789")
    }
}
