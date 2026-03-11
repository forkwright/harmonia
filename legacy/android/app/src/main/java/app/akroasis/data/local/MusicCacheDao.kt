// DAO for music metadata cache with TTL expiration
package app.akroasis.data.local

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface MusicCacheDao {
    @Query("SELECT * FROM track_cache WHERE id = :trackId AND cachedAt > :expiryTime")
    suspend fun getTrack(trackId: String, expiryTime: Long): TrackCacheEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertTrack(track: TrackCacheEntity)

    @Query("DELETE FROM track_cache WHERE cachedAt < :expiryTime")
    suspend fun deleteExpired(expiryTime: Long)

    @Query("DELETE FROM track_cache")
    suspend fun clearAll()
}
