// Focus filtering data model for POST /api/v3/library/filter
package app.akroasis.data.model

import com.google.gson.annotations.SerializedName

enum class FilterField {
    @SerializedName("format")
    FORMAT,

    @SerializedName("sampleRate")
    SAMPLE_RATE,

    @SerializedName("bitDepth")
    BIT_DEPTH,

    @SerializedName("codec")
    CODEC,

    @SerializedName("bitrate")
    BITRATE,

    @SerializedName("dynamicRange")
    DYNAMIC_RANGE,

    @SerializedName("lossless")
    LOSSLESS,

    @SerializedName("artist")
    ARTIST,

    @SerializedName("album")
    ALBUM,

    @SerializedName("genre")
    GENRE,

    @SerializedName("year")
    YEAR
}

enum class FilterOperator {
    @SerializedName("equals")
    EQUALS,

    @SerializedName("notEquals")
    NOT_EQUALS,

    @SerializedName("greaterThan")
    GREATER_THAN,

    @SerializedName("lessThan")
    LESS_THAN,

    @SerializedName("greaterThanOrEqual")
    GREATER_THAN_OR_EQUAL,

    @SerializedName("lessThanOrEqual")
    LESS_THAN_OR_EQUAL,

    @SerializedName("contains")
    CONTAINS,

    @SerializedName("notContains")
    NOT_CONTAINS,

    @SerializedName("in")
    IN,

    @SerializedName("notIn")
    NOT_IN
}

enum class FilterLogic {
    @SerializedName("AND")
    AND,

    @SerializedName("OR")
    OR
}

data class FilterRule(
    @SerializedName("field")
    val field: FilterField,

    @SerializedName("operator")
    val operator: FilterOperator,

    @SerializedName("value")
    val value: Any                     // String, Int, Boolean depending on field
)

data class FilterRequest(
    @SerializedName("conditions")
    val conditions: List<FilterRule>,

    @SerializedName("logic")
    val logic: FilterLogic = FilterLogic.AND,

    @SerializedName("page")
    val page: Int = 1,

    @SerializedName("pageSize")
    val pageSize: Int = 50
)

data class FilterResponse(
    @SerializedName("tracks")
    val tracks: List<Track>,

    @SerializedName("totalCount")
    val totalCount: Int,

    @SerializedName("page")
    val page: Int,

    @SerializedName("pageSize")
    val pageSize: Int,

    @SerializedName("summary")
    val summary: FilterSummary?
)

data class FilterSummary(
    @SerializedName("avgDynamicRange")
    val avgDynamicRange: Double?,

    @SerializedName("formatDistribution")
    val formatDistribution: Map<String, Int>,

    @SerializedName("losslessCount")
    val losslessCount: Int,

    @SerializedName("totalDuration")
    val totalDuration: Long?
)

data class LibraryFacets(
    @SerializedName("formats")
    val formats: List<String>,

    @SerializedName("sampleRates")
    val sampleRates: List<Int>,

    @SerializedName("bitDepths")
    val bitDepths: List<Int>,

    @SerializedName("genres")
    val genres: List<String>,

    @SerializedName("years")
    val years: List<Int>,

    @SerializedName("dynamicRangeRange")
    val dynamicRangeRange: DynamicRangeRange,

    @SerializedName("codecList")
    val codecList: List<String>
)

data class DynamicRangeRange(
    @SerializedName("min")
    val min: Int,

    @SerializedName("max")
    val max: Int
)

data class SmartPlaylist(
    @SerializedName("id")
    val id: String,

    @SerializedName("name")
    val name: String,

    @SerializedName("filterRequest")
    val filterRequest: FilterRequest,

    @SerializedName("trackCount")
    val trackCount: Int,

    @SerializedName("lastRefreshed")
    val lastRefreshed: String,

    @SerializedName("createdAt")
    val createdAt: String,

    @SerializedName("updatedAt")
    val updatedAt: String
)

data class CreateSmartPlaylistRequest(
    @SerializedName("name")
    val name: String,

    @SerializedName("filterRequest")
    val filterRequest: FilterRequest
)

data class UpdateSmartPlaylistRequest(
    @SerializedName("name")
    val name: String?,

    @SerializedName("filterRequest")
    val filterRequest: FilterRequest?
)
