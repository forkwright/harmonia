package app.akroasis.audio

import app.akroasis.data.model.Track
import app.akroasis.data.repository.MusicRepository
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Before
import org.junit.Test
import org.mockito.kotlin.mock
import org.mockito.kotlin.whenever
import kotlin.test.assertTrue
import kotlin.test.assertEquals

@OptIn(ExperimentalCoroutinesApi::class)
class TrackLoaderTest {

    private lateinit var trackLoader: TrackLoader
    private lateinit var mockRepository: MusicRepository

    private val testTrack = Track(
        id = "1",
        title = "Test Track",
        artist = "Test Artist",
        album = "Test Album",
        duration = 300000L,
        format = "FLAC",
        bitrate = 1411,
        trackNumber = 1
    )

    @Before
    fun setup() {
        mockRepository = mock()
        trackLoader = TrackLoader(mockRepository)
    }

    @Test
    fun `unsupported format returns UnsupportedFormat error`() = runTest {
        val mp3Track = testTrack.copy(format = "MP3")
        whenever(mockRepository.getTrack("1")).thenReturn(Result.success(mp3Track))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.UnsupportedFormat)
        assertEquals("Unsupported format: MP3. Only FLAC is currently supported.", exception.message)
    }

    @Test
    fun `file size validation blocks oversized files`() = runTest {
        whenever(mockRepository.getTrack("1")).thenReturn(Result.success(testTrack))

        val oversizedBody = mock<ResponseBody>()
        whenever(oversizedBody.contentLength()).thenReturn(600L * 1024 * 1024)

        whenever(mockRepository.streamTrack("1")).thenReturn(Result.success(oversizedBody))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.FileSizeError)
        assertTrue(exception.message!!.contains("600MB"))
    }

    @Test
    fun `network error returns NetworkError`() = runTest {
        val networkException = Exception("Connection timeout")
        whenever(mockRepository.getTrack("1")).thenReturn(Result.failure(networkException))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.NetworkError)
        assertTrue(exception.message!!.contains("Failed to fetch track"))
    }

    @Test
    fun `successful FLAC load validates format`() = runTest {
        whenever(mockRepository.getTrack("1")).thenReturn(Result.success(testTrack))

        val validBody = mock<ResponseBody>()
        whenever(validBody.contentLength()).thenReturn(10L * 1024 * 1024)

        val flacData = ByteArray(1024)
        whenever(validBody.bytes()).thenReturn(flacData)

        whenever(mockRepository.streamTrack("1")).thenReturn(Result.success(validBody))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.DecodeError)
    }

    @Test
    fun `null track from repository returns NetworkError`() = runTest {
        whenever(mockRepository.getTrack("1")).thenReturn(Result.success(null))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.NetworkError)
    }

    @Test
    fun `streaming failure returns NetworkError`() = runTest {
        whenever(mockRepository.getTrack("1")).thenReturn(Result.success(testTrack))

        val streamException = Exception("Stream unavailable")
        whenever(mockRepository.streamTrack("1")).thenReturn(Result.failure(streamException))

        val result = trackLoader.loadAndDecodeTrack("1")

        assertTrue(result.isFailure)
        val exception = result.exceptionOrNull()
        assertTrue(exception is LoadError.NetworkError)
        assertTrue(exception.message!!.contains("Failed to stream track"))
    }
}
