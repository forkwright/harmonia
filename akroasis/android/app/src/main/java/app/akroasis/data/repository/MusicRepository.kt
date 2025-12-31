// Music repository with caching and network resilience
package app.akroasis.data.repository

import app.akroasis.data.api.MouseionApi
import app.akroasis.data.local.MusicCacheDao
import app.akroasis.data.local.TrackCacheEntity
import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.Track
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.ResponseBody
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MusicRepository @Inject constructor(
    private val api: MouseionApi,
    private val cacheDao: MusicCacheDao
) {
    companion object {
        private const val CACHE_TTL_MS = 60 * 60 * 1000L // 1 hour
    }

    suspend fun getArtists(
        page: Int = 1,
        pageSize: Int = 50
    ): Result<List<Artist>> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getArtists(page, pageSize)
                if (response.isSuccessful && response.body() != null) {
                    Result.success(response.body()!!)
                } else {
                    Result.failure(Exception("Failed to fetch artists: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getAlbums(
        artistId: String? = null,
        page: Int = 1,
        pageSize: Int = 50
    ): Result<List<Album>> = withContext(Dispatchers.IO) {
        try {
            val response = api.getAlbums(artistId, page, pageSize)
            if (response.isSuccessful && response.body() != null) {
                Result.success(response.body()!!)
            } else {
                Result.failure(Exception("Failed to fetch albums: ${response.code()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getTracks(
        albumId: String? = null,
        artistId: String? = null,
        page: Int = 1,
        pageSize: Int = 100
    ): Result<List<Track>> = withContext(Dispatchers.IO) {
        try {
            val response = api.getTracks(albumId, artistId, page, pageSize)
            if (response.isSuccessful && response.body() != null) {
                Result.success(response.body()!!)
            } else {
                Result.failure(Exception("Failed to fetch tracks: ${response.code()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getTrack(trackId: String): Result<Track> = withContext(Dispatchers.IO) {
        try {
            // Check cache first
            val expiryTime = System.currentTimeMillis() - CACHE_TTL_MS
            val cachedTrack = cacheDao.getTrack(trackId, expiryTime)
            if (cachedTrack != null) {
                return@withContext Result.success(cachedTrack.toTrack())
            }

            // Cache miss - fetch from network with retry
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getTrack(trackId)
                if (response.isSuccessful && response.body() != null) {
                    val track = response.body()!!
                    // Cache successful response
                    cacheDao.insertTrack(TrackCacheEntity.fromTrack(track))
                    Result.success(track)
                } else {
                    Result.failure(Exception("Failed to fetch track: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun streamTrack(
        trackId: String,
        range: String? = null
    ): Result<ResponseBody> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.streamTrack(trackId, range)
                if (response.isSuccessful && response.body() != null) {
                    Result.success(response.body()!!)
                } else {
                    Result.failure(Exception("Failed to stream track: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
