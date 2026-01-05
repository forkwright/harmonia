// DAO for smart playlist database operations
package app.akroasis.data.local

import androidx.room.*
import kotlinx.coroutines.flow.Flow

@Dao
interface SmartPlaylistDao {
    @Query("SELECT * FROM smart_playlists ORDER BY name ASC")
    fun getAllPlaylists(): Flow<List<SmartPlaylistEntity>>

    @Query("SELECT * FROM smart_playlists WHERE id = :id")
    suspend fun getPlaylist(id: String): SmartPlaylistEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertPlaylist(playlist: SmartPlaylistEntity)

    @Update
    suspend fun updatePlaylist(playlist: SmartPlaylistEntity)

    @Delete
    suspend fun deletePlaylist(playlist: SmartPlaylistEntity)

    @Query("DELETE FROM smart_playlists WHERE id = :id")
    suspend fun deleteById(id: String)

    @Query("UPDATE smart_playlists SET lastRefreshed = :timestamp, trackCount = :trackCount WHERE id = :id")
    suspend fun updateRefreshStatus(id: String, timestamp: Long, trackCount: Int)

    @Query("SELECT * FROM smart_playlists WHERE autoRefresh = 1")
    suspend fun getAutoRefreshPlaylists(): List<SmartPlaylistEntity>

    @Query("DELETE FROM smart_playlists")
    suspend fun clearAll()
}
