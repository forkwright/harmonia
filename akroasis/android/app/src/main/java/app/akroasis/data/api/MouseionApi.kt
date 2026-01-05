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
