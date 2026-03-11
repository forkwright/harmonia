// Smart playlist entity for Room database
package app.akroasis.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey
import androidx.room.TypeConverter
import androidx.room.TypeConverters
import app.akroasis.data.model.FilterRequest
import com.google.gson.Gson

@Entity(tableName = "smart_playlists")
@TypeConverters(SmartPlaylistConverters::class)
data class SmartPlaylistEntity(
    @PrimaryKey
    val id: String,

    val name: String,

    val filterRequest: FilterRequest,

    val trackCount: Int,

    val lastRefreshed: Long,

    val autoRefresh: Boolean = true,

    val createdAt: Long,

    val updatedAt: Long
)

class SmartPlaylistConverters {
    private val gson = Gson()

    @TypeConverter
    fun fromFilterRequest(value: FilterRequest): String {
        return gson.toJson(value)
    }

    @TypeConverter
    fun toFilterRequest(value: String): FilterRequest {
        return gson.fromJson(value, FilterRequest::class.java)
    }
}
