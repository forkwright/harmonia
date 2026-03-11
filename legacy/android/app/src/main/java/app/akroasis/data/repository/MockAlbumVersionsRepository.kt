// Mock album versions repository for Phase 2 UI testing
package app.akroasis.data.repository

import app.akroasis.data.model.AlbumVersion
import app.akroasis.data.model.AlbumVersionsResponse
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MockAlbumVersionsRepository @Inject constructor() {

    suspend fun getAlbumVersions(albumId: Int): Result<AlbumVersionsResponse> {
        kotlinx.coroutines.delay(300)

        val mockResponse = AlbumVersionsResponse(
            canonical = AlbumVersion(
                id = albumId,
                title = "Dark Side of the Moon",
                releaseGroupMbid = "f5093c06-23e3-404f-aeaa-40f72885ee3a",
                releaseDate = "1973-03-01",
                edition = "Original",
                country = "UK",
                label = "Harvest",
                format = "Vinyl",
                trackCount = 10,
                averageDynamicRange = 12.5f,
                lossless = true
            ),
            versions = listOf(
                AlbumVersion(
                    id = albumId + 1,
                    title = "Dark Side of the Moon (2011 Remaster)",
                    releaseGroupMbid = "f5093c06-23e3-404f-aeaa-40f72885ee3a",
                    releaseDate = "2011-09-26",
                    edition = "Remaster",
                    country = "US",
                    label = "Capitol",
                    format = "Digital",
                    trackCount = 10,
                    averageDynamicRange = 8.2f,  // Loudness war victim
                    lossless = true
                ),
                AlbumVersion(
                    id = albumId + 2,
                    title = "Dark Side of the Moon (SACD)",
                    releaseGroupMbid = "f5093c06-23e3-404f-aeaa-40f72885ee3a",
                    releaseDate = "2003-11-24",
                    edition = "SACD",
                    country = "US",
                    label = "Capitol",
                    format = "SACD",
                    trackCount = 10,
                    averageDynamicRange = 13.8f,
                    lossless = true
                )
            )
        )

        return Result.success(mockResponse)
    }
}
