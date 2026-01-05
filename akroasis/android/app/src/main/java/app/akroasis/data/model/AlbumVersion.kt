// Album version/edition data for GET /api/v3/albums/{id}/versions
package app.akroasis.data.model

data class AlbumVersionsResponse(
    val canonical: AlbumVersion,      // Primary/canonical version
    val versions: List<AlbumVersion>  // All versions/editions
)

data class AlbumVersion(
    val id: Int,
    val title: String,
    val releaseGroupMbid: String?,
    val releaseDate: String?,
    val edition: String?,             // "Original", "Remaster", "Deluxe", etc.
    val country: String?,
    val label: String?,
    val format: String?,              // "CD", "Vinyl", "Digital"
    val trackCount: Int,
    val averageDynamicRange: Float?,  // Average DR across all tracks
    val lossless: Boolean
)
