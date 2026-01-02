// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Qualities;

public static class QualityFinder
{
    public static Quality FindBySourceAndResolution(QualitySource source, int resolution, Modifier modifier, ILogger? logger = null)
    {
        // Check for a perfect 3-way match
        var matchingQuality = Quality.All.SingleOrDefault(q => q.Source == source && q.Resolution == resolution && q.Modifier == modifier);

        if (matchingQuality is not null)
        {
            return matchingQuality;
        }

        // Check for Source and Modifier Match for Qualities with Unknown Resolution
        var matchingQualitiesUnknownResolution = Quality.All
            .Where(q => q.Source == source && q.Resolution == 0 && q.Modifier == modifier && !Equals(q, Quality.Unknown))
            .ToList();

        if (matchingQualitiesUnknownResolution.Any())
        {
            if (matchingQualitiesUnknownResolution.Count == 1)
            {
                return matchingQualitiesUnknownResolution.First();
            }

            foreach (var quality in matchingQualitiesUnknownResolution)
            {
                if (quality.Source >= source)
                {
                    logger?.LogWarning("Unable to find exact quality for {Source}, {Resolution}, and {Modifier}. Using {Quality} as fallback",
                        source, resolution, modifier, quality);
                    return quality;
                }
            }
        }

        // Check for Modifier match
        var matchingModifier = Quality.All.Where(q => q.Modifier == modifier);

        var matchingResolution = matchingModifier
            .Where(q => q.Resolution == resolution)
            .OrderBy(q => q.Source)
            .ToList();

        var nearestQuality = Quality.Unknown;

        foreach (var quality in matchingResolution)
        {
            if (quality.Source >= source)
            {
                nearestQuality = quality;
                break;
            }
        }

        logger?.LogWarning("Unable to find exact quality for {Source}, {Resolution}, and {Modifier}. Using {Quality} as fallback",
            source, resolution, modifier, nearestQuality);

        return nearestQuality;
    }
}
