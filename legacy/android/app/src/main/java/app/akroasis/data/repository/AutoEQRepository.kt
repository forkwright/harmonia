// AutoEQ profile repository
package app.akroasis.data.repository

import app.akroasis.data.model.AutoEQProfile
import app.akroasis.data.model.FilterType
import app.akroasis.data.model.ParametricBand
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AutoEQRepository @Inject constructor() {

    fun getAvailableProfiles(): List<AutoEQProfile> {
        return listOf(
            AutoEQProfile(
                manufacturer = "Sennheiser",
                model = "HD 600",
                parametricEq = listOf(
                    ParametricBand(FilterType.PEAKING, 20f, -2.4f, 1.1f),
                    ParametricBand(FilterType.LOW_SHELF, 105f, 5.6f, 0.7f),
                    ParametricBand(FilterType.PEAKING, 730f, -2.2f, 0.9f),
                    ParametricBand(FilterType.PEAKING, 2200f, 3.3f, 2.5f),
                    ParametricBand(FilterType.PEAKING, 3100f, -3.9f, 3.0f),
                    ParametricBand(FilterType.PEAKING, 5500f, -1.9f, 4.5f),
                    ParametricBand(FilterType.HIGH_SHELF, 10000f, -4.2f, 0.7f)
                )
            ),
            AutoEQProfile(
                manufacturer = "Sennheiser",
                model = "HD 650",
                parametricEq = listOf(
                    ParametricBand(FilterType.PEAKING, 18f, -0.9f, 0.6f),
                    ParametricBand(FilterType.LOW_SHELF, 105f, 6.5f, 0.7f),
                    ParametricBand(FilterType.PEAKING, 155f, -0.7f, 1.4f),
                    ParametricBand(FilterType.PEAKING, 4300f, -2.6f, 4.6f),
                    ParametricBand(FilterType.PEAKING, 5900f, -4.4f, 2.7f),
                    ParametricBand(FilterType.HIGH_SHELF, 10000f, -5.0f, 0.7f)
                )
            ),
            AutoEQProfile(
                manufacturer = "Beyerdynamic",
                model = "DT 770 Pro 80 Ohm",
                parametricEq = listOf(
                    ParametricBand(FilterType.PEAKING, 27f, -3.4f, 0.4f),
                    ParametricBand(FilterType.LOW_SHELF, 105f, 5.7f, 0.7f),
                    ParametricBand(FilterType.PEAKING, 215f, 2.9f, 2.0f),
                    ParametricBand(FilterType.PEAKING, 3600f, 2.0f, 2.0f),
                    ParametricBand(FilterType.PEAKING, 5900f, -4.1f, 3.5f),
                    ParametricBand(FilterType.PEAKING, 8700f, -3.5f, 1.7f),
                    ParametricBand(FilterType.HIGH_SHELF, 10000f, -5.4f, 0.7f)
                )
            ),
            AutoEQProfile(
                manufacturer = "Audio-Technica",
                model = "ATH-M50x",
                parametricEq = listOf(
                    ParametricBand(FilterType.PEAKING, 22f, -2.8f, 0.7f),
                    ParametricBand(FilterType.LOW_SHELF, 105f, 4.9f, 0.7f),
                    ParametricBand(FilterType.PEAKING, 340f, -3.1f, 2.1f),
                    ParametricBand(FilterType.PEAKING, 730f, 2.6f, 1.2f),
                    ParametricBand(FilterType.PEAKING, 3400f, 3.5f, 3.7f),
                    ParametricBand(FilterType.PEAKING, 8700f, -7.5f, 2.5f),
                    ParametricBand(FilterType.HIGH_SHELF, 10000f, -3.9f, 0.7f)
                )
            )
        )
    }

    fun searchProfiles(query: String): List<AutoEQProfile> {
        if (query.isBlank()) return getAvailableProfiles()

        val lowerQuery = query.lowercase()
        return getAvailableProfiles().filter { profile ->
            profile.fullName.lowercase().contains(lowerQuery) ||
            profile.manufacturer.lowercase().contains(lowerQuery) ||
            profile.model.lowercase().contains(lowerQuery)
        }
    }

    fun getProfileByName(fullName: String): AutoEQProfile? {
        return getAvailableProfiles().find { it.fullName == fullName }
    }
}
