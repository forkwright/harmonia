// DAO for media progress tracking
package app.akroasis.data.local

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface MediaProgressDao {
    @Query("SELECT * FROM media_progress ORDER BY lastPlayedAt DESC LIMIT :limit")
    suspend fun getRecentProgress(limit: Int = 20): List<MediaProgressEntity>

    @Query("SELECT * FROM media_progress WHERE mediaItemId = :id")
    suspend fun getProgress(id: String): MediaProgressEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun saveProgress(progress: MediaProgressEntity)

    @Query("DELETE FROM media_progress WHERE mediaItemId = :id")
    suspend fun deleteProgress(id: String)

    @Query("DELETE FROM media_progress WHERE isComplete = 1 AND lastPlayedAt < :expiryTime")
    suspend fun deleteOldCompleted(expiryTime: Long)

    @Query("DELETE FROM media_progress")
    suspend fun clearAll()
}
