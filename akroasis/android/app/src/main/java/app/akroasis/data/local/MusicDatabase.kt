// Room database for music metadata caching
package app.akroasis.data.local

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(
    entities = [TrackCacheEntity::class],
    version = 1,
    exportSchema = false
)
abstract class MusicDatabase : RoomDatabase() {
    abstract fun musicCacheDao(): MusicCacheDao
}
