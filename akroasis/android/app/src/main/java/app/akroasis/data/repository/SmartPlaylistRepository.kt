// Smart playlist repository with server sync
package app.akroasis.data.repository

import app.akroasis.data.api.MouseionApi
import app.akroasis.data.local.SmartPlaylistDao
import app.akroasis.data.local.SmartPlaylistEntity
import app.akroasis.data.model.*
import app.akroasis.data.network.RetryPolicy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import java.time.Instant
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SmartPlaylistRepository @Inject constructor(
    private val api: MouseionApi,
    private val dao: SmartPlaylistDao
) {

    /**
     * Get all smart playlists from local database (reactive)
     */
    fun getAllPlaylists(): Flow<List<SmartPlaylistEntity>> {
        return dao.getAllPlaylists()
    }

    /**
     * Sync playlists from Mouseion to local database
     */
    suspend fun syncFromServer(): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.getAllSmartPlaylists()
                response.body()?.let { playlists ->
                    // Insert/update all in local database
                    playlists.forEach { playlist ->
                        dao.insertPlaylist(playlist.toEntity())
                    }
                    Result.success(Unit)
                } ?: Result.failure(Exception("Sync failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Create new smart playlist
     */
    suspend fun createPlaylist(
        name: String,
        filterRequest: FilterRequest
    ): Result<SmartPlaylist> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val request = CreateSmartPlaylistRequest(name, filterRequest)
                val response = api.createSmartPlaylist(request)

                response.body()?.let { playlist ->
                    // Save to local database
                    dao.insertPlaylist(playlist.toEntity())
                    Result.success(playlist)
                } ?: Result.failure(Exception("Create failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Update smart playlist
     */
    suspend fun updatePlaylist(
        id: String,
        name: String? = null,
        filterRequest: FilterRequest? = null
    ): Result<SmartPlaylist> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val request = UpdateSmartPlaylistRequest(name, filterRequest)
                val response = api.updateSmartPlaylist(id, request)

                response.body()?.let { playlist ->
                    // Update local database
                    dao.insertPlaylist(playlist.toEntity())
                    Result.success(playlist)
                } ?: Result.failure(Exception("Update failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Delete smart playlist
     */
    suspend fun deletePlaylist(id: String): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.deleteSmartPlaylist(id)

                if (response.isSuccessful) {
                    // Delete from local database
                    dao.deleteById(id)

                    Result.success(Unit)
                } else {
                    Result.failure(Exception("Delete failed: ${response.code()}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Refresh smart playlist (recalculate tracks)
     */
    suspend fun refreshPlaylist(id: String): Result<SmartPlaylist> = withContext(Dispatchers.IO) {
        try {
            RetryPolicy.retryWithExponentialBackoff {
                val response = api.refreshSmartPlaylist(id)

                response.body()?.let { playlist ->
                    // Update local cache with new track count
                    dao.updateRefreshStatus(
                        id = id,
                        timestamp = System.currentTimeMillis(),
                        trackCount = playlist.trackCount
                    )
                    Result.success(playlist)
                } ?: Result.failure(Exception("Refresh failed: ${response.code()} - empty body"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Auto-refresh all playlists marked for auto-refresh
     * Called after library scan
     */
    suspend fun autoRefreshAll(): Result<Int> = withContext(Dispatchers.IO) {
        try {
            val autoRefreshPlaylists = dao.getAutoRefreshPlaylists()
            var refreshedCount = 0

            autoRefreshPlaylists.forEach { playlist ->
                val result = refreshPlaylist(playlist.id)
                if (result.isSuccess) {
                    refreshedCount++
                }
            }

            Result.success(refreshedCount)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get playlist details from local database
     */
    suspend fun getPlaylist(id: String): SmartPlaylistEntity? = withContext(Dispatchers.IO) {
        dao.getPlaylist(id)
    }
}

// Extension function to convert API model to Entity
private fun SmartPlaylist.toEntity(): SmartPlaylistEntity {
    return SmartPlaylistEntity(
        id = id,
        name = name,
        filterRequest = filterRequest,
        trackCount = trackCount,
        lastRefreshed = parseIso8601(lastRefreshed),
        autoRefresh = true,  // Default
        createdAt = parseIso8601(createdAt),
        updatedAt = parseIso8601(updatedAt)
    )
}

private fun parseIso8601(dateString: String): Long {
    return try {
        Instant.parse(dateString).toEpochMilli()
    } catch (e: Exception) {
        System.currentTimeMillis()
    }
}
