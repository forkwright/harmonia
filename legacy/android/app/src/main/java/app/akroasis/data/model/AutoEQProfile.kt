// AutoEQ headphone compensation profile
package app.akroasis.data.model

data class AutoEQProfile(
    val manufacturer: String,
    val model: String,
    val parametricEq: List<ParametricBand>
) {
    val fullName: String
        get() = "$manufacturer $model"
}

data class ParametricBand(
    val type: FilterType,
    val frequency: Float,
    val gain: Float,
    val q: Float
)

enum class FilterType {
    PEAKING,
    LOW_SHELF,
    HIGH_SHELF,
    LOW_PASS,
    HIGH_PASS
}
