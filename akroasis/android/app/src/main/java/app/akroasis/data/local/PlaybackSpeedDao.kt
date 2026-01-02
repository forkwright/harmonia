// Playback speed database access
package app.akroasis.data.local

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface PlaybackSpeedDao {
    @Query("SELECT * FROM playback_speeds WHERE contentId = :contentId")
    suspend fun getSpeed(contentId: String): PlaybackSpeedRecord?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun setSpeed(record: PlaybackSpeedRecord)

    @Query("DELETE FROM playback_speeds WHERE contentId = :contentId")
    suspend fun deleteSpeed(contentId: String)

    @Query("DELETE FROM playback_speeds")
    suspend fun clearAll()
}
