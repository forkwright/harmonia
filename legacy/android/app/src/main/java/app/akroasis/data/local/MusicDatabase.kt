// Room database for music metadata caching
package app.akroasis.data.local

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(
    entities = [
        TrackCacheEntity::class,
        PlaybackSpeedRecord::class,
        SmartPlaylistEntity::class,
        MediaProgressEntity::class
    ],
    version = 4,
    exportSchema = false
)
abstract class MusicDatabase : RoomDatabase() {
    abstract fun musicCacheDao(): MusicCacheDao
    abstract fun playbackSpeedDao(): PlaybackSpeedDao
    abstract fun smartPlaylistDao(): SmartPlaylistDao
    abstract fun mediaProgressDao(): MediaProgressDao
}
