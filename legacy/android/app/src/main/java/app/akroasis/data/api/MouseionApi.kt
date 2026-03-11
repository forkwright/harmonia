// Retrofit API interface for Mouseion backend
package app.akroasis.data.api

import app.akroasis.data.model.Album
import app.akroasis.data.model.Artist
import app.akroasis.data.model.Track
import okhttp3.ResponseBody
import retrofit2.Response
import retrofit2.http.*

interface MouseionApi {

    @POST("api/v3/login")
    suspend fun login(
        @Body credentials: LoginRequest
    ): Response<LoginResponse>

    @POST("api/v3/refresh")
    suspend fun refreshToken(
        @Body request: RefreshTokenRequest
    ): Response<LoginResponse>

    @GET("api/v3/music/artists")
    suspend fun getArtists(
        @Query("page") page: Int = 1,
        @Query("pageSize") pageSize: Int = 50,
        @Query("sort") sort: String = "name"
    ): Response<List<Artist>>

    @GET("api/v3/music/albums")
    suspend fun getAlbums(
        @Query("artistId") artistId: String? = null,
        @Query("page") page: Int = 1,
        @Query("pageSize") pageSize: Int = 50,
        @Query("sort") sort: String = "title"
    ): Response<List<Album>>

    @GET("api/v3/music/tracks")
    suspend fun getTracks(
        @Query("albumId") albumId: String? = null,
        @Query("artistId") artistId: String? = null,
        @Query("page") page: Int = 1,
        @Query("pageSize") pageSize: Int = 100,
        @Query("sort") sort: String = "trackNumber"
    ): Response<List<Track>>

    @GET("api/v3/music/tracks/{trackId}")
    suspend fun getTrack(
        @Path("trackId") trackId: String
    ): Response<Track>

    @Streaming
    @GET("api/v3/stream/{trackId}")
    suspend fun streamTrack(
        @Path("trackId") trackId: String,
        @Header("Range") range: String? = null
    ): Response<ResponseBody>

    @GET("api/v3/search")
    suspend fun search(
        @Query("q") query: String,
        @Query("limit") limit: Int = 50
    ): Response<List<app.akroasis.data.model.SearchResult>>

    @POST("api/v3/library/filter")
    suspend fun filterLibrary(
        @Body request: app.akroasis.data.model.FilterRequest
    ): Response<app.akroasis.data.model.FilterResponse>

    @GET("api/v3/library/facets")
    suspend fun getLibraryFacets(): Response<app.akroasis.data.model.LibraryFacets>

    @POST("api/v3/playlists/smart")
    suspend fun createSmartPlaylist(
        @Body request: app.akroasis.data.model.CreateSmartPlaylistRequest
    ): Response<app.akroasis.data.model.SmartPlaylist>

    @GET("api/v3/playlists/smart/{id}")
    suspend fun getSmartPlaylist(
        @Path("id") id: String
    ): Response<app.akroasis.data.model.SmartPlaylist>

    @PUT("api/v3/playlists/smart/{id}")
    suspend fun updateSmartPlaylist(
        @Path("id") id: String,
        @Body request: app.akroasis.data.model.UpdateSmartPlaylistRequest
    ): Response<app.akroasis.data.model.SmartPlaylist>

    @DELETE("api/v3/playlists/smart/{id}")
    suspend fun deleteSmartPlaylist(
        @Path("id") id: String
    ): Response<Unit>

    @POST("api/v3/playlists/smart/{id}/refresh")
    suspend fun refreshSmartPlaylist(
        @Path("id") id: String
    ): Response<app.akroasis.data.model.SmartPlaylist>

    @GET("api/v3/playlists/smart")
    suspend fun getAllSmartPlaylists(): Response<List<app.akroasis.data.model.SmartPlaylist>>

    // Audiobooks
    @GET("api/v3/audiobooks")
    suspend fun getAudiobooks(
        @Query("page") page: Int = 1,
        @Query("pageSize") pageSize: Int = 50
    ): Response<List<AudiobookDto>>

    @GET("api/v3/audiobooks/{id}")
    suspend fun getAudiobook(
        @Path("id") id: String
    ): Response<AudiobookDto>

    @GET("api/v3/chapters/{mediaFileId}")
    suspend fun getChapters(
        @Path("mediaFileId") fileId: String
    ): Response<List<app.akroasis.data.model.Chapter>>

    // Ebooks
    @GET("api/v3/books")
    suspend fun getEbooks(
        @Query("page") page: Int = 1,
        @Query("pageSize") pageSize: Int = 50
    ): Response<List<EbookDto>>

    @GET("api/v3/books/{id}")
    suspend fun getEbook(
        @Path("id") id: String
    ): Response<EbookDto>

    // Progress tracking
    @GET("api/v3/continue")
    suspend fun getContinueFeed(
        @Query("userId") userId: String = "default",
        @Query("limit") limit: Int = 20
    ): Response<List<ContinueItemDto>>

    @POST("api/v3/progress")
    suspend fun updateProgress(
        @Body request: ProgressUpdateRequest
    ): Response<Unit>

    @GET("api/v3/progress/{mediaItemId}")
    suspend fun getProgress(
        @Path("mediaItemId") id: String,
        @Query("userId") userId: String = "default"
    ): Response<app.akroasis.data.model.MediaProgress>

    @DELETE("api/v3/progress/{mediaItemId}")
    suspend fun deleteProgress(
        @Path("mediaItemId") id: String,
        @Query("userId") userId: String = "default"
    ): Response<Unit>

    // Sessions
    @GET("api/v3/sessions")
    suspend fun getSessions(
        @Query("userId") userId: String = "default",
        @Query("activeOnly") activeOnly: Boolean = false,
        @Query("limit") limit: Int = 100
    ): Response<List<SessionDto>>

    @GET("api/v3/sessions/{sessionId}")
    suspend fun getSession(
        @Path("sessionId") id: String
    ): Response<SessionDto>

    @POST("api/v3/sessions")
    suspend fun startSession(
        @Body request: StartSessionRequest
    ): Response<SessionDto>

    @PUT("api/v3/sessions/{sessionId}")
    suspend fun updateSession(
        @Path("sessionId") id: String,
        @Body request: UpdateSessionRequest
    ): Response<Unit>

    @DELETE("api/v3/sessions/{sessionId}")
    suspend fun deleteSession(
        @Path("sessionId") id: String
    ): Response<Unit>
}

data class LoginRequest(
    val username: String,
    val password: String
)

data class LoginResponse(
    val accessToken: String,
    val refreshToken: String,
    val expiresIn: Long,
    val userId: String
)

data class RefreshTokenRequest(
    val refreshToken: String
)

data class SearchResults(
    val artists: List<Artist>,
    val albums: List<Album>,
    val tracks: List<Track>
)

data class AudiobookDto(
    val id: String,
    val title: String,
    val author: String,
    val narrator: String?,
    val seriesName: String?,
    val seriesNumber: Int?,
    val duration: Long,
    val coverArtUrl: String?,
    val totalChapters: Int,
    val format: String,
    val fileSize: Long,
    val filePath: String,
    val createdAt: String,
    val updatedAt: String
)

data class EbookDto(
    val id: String,
    val title: String,
    val author: String,
    val seriesName: String?,
    val seriesNumber: Int?,
    val pageCount: Int?,
    val publishDate: String?,
    val coverArtUrl: String?,
    val format: String,
    val fileSize: Long,
    val filePath: String,
    val createdAt: String,
    val updatedAt: String
)

data class ContinueItemDto(
    val mediaItem: MediaItemDto,
    val progress: app.akroasis.data.model.MediaProgress
)

data class MediaItemDto(
    val id: String,
    val title: String,
    val mediaType: String,
    val coverArtUrl: String?,
    val duration: Long?,
    val author: String?,
    val artist: String?,
    val album: String?
)

data class ProgressUpdateRequest(
    val mediaItemId: String,
    val mediaType: String,
    val positionMs: Long,
    val totalDurationMs: Long?,
    val userId: String = "default"
)

data class SessionDto(
    val id: String,
    val userId: String,
    val mediaItemId: String,
    val mediaType: String,
    val startedAt: Long,
    val endedAt: Long?,
    val durationMs: Long,
    val isActive: Boolean
)

data class StartSessionRequest(
    val mediaItemId: String,
    val mediaType: String,
    val userId: String = "default"
)

data class UpdateSessionRequest(
    val endedAt: Long?,
    val durationMs: Long
)
