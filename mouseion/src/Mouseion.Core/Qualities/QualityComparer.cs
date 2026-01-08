// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Qualities;

public static class QualityComparer
{
    /// <summary>
    /// Compares two QualityModels to determine upgrade order.
    /// Returns: positive if left > right, negative if left < right, 0 if equal
    /// </summary>
    public static int Compare(QualityModel left, QualityModel right)
    {
        var leftWeight = GetWeight(left.Quality);
        var rightWeight = GetWeight(right.Quality);

        // Compare quality weights first
        var weightComparison = leftWeight.CompareTo(rightWeight);
        if (weightComparison != 0)
        {
            return weightComparison;
        }

        // If qualities are equal, compare revisions
        return left.Revision.CompareTo(right.Revision);
    }

    /// <summary>
    /// Checks if new quality is an upgrade over current quality.
    /// </summary>
    public static bool IsUpgrade(QualityModel? current, QualityModel candidate)
    {
        // No existing quality = always an upgrade
        if (current == null)
        {
            return true;
        }

        // Compare qualities
        return Compare(candidate, current) > 0;
    }

    /// <summary>
    /// Checks if quality meets or exceeds minimum requirement.
    /// </summary>
    public static bool MeetsMinimum(QualityModel quality, QualityModel minimum)
    {
        return Compare(quality, minimum) >= 0;
    }

    /// <summary>
    /// Checks if quality has reached cutoff (no further upgrades needed).
    /// </summary>
    public static bool HasReachedCutoff(QualityModel quality, QualityModel cutoff)
    {
        return Compare(quality, cutoff) >= 0;
    }

    /// <summary>
    /// Gets weight for a quality from default definitions.
    /// Higher weight = better quality.
    /// </summary>
    private static int GetWeight(Quality quality)
    {
        var definition = Quality.DefaultQualityDefinitions
            .FirstOrDefault(d => d.Quality.Id == quality.Id);

        // Return weight if found, otherwise use quality ID as fallback
        return definition?.Weight ?? quality.Id;
    }
}
