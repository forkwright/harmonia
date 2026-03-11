// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public class PodcastEpisode : ModelBase
{
    public int PodcastShowId { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? Description { get; set; }
    public string? EpisodeGuid { get; set; }
    public int? EpisodeNumber { get; set; }
    public int? SeasonNumber { get; set; }
    public DateTime? PublishDate { get; set; }
    public int? Duration { get; set; }
    public string? EnclosureUrl { get; set; }
    public long? EnclosureLength { get; set; }
    public string? EnclosureType { get; set; }
    public string? ImageUrl { get; set; }
    public bool Explicit { get; set; }
    public bool Monitored { get; set; } = true;
    public DateTime Added { get; set; }

    public override string ToString()
    {
        if (EpisodeNumber.HasValue && SeasonNumber.HasValue)
        {
            return $"S{SeasonNumber:00}E{EpisodeNumber:00} - {Title}";
        }

        return Title;
    }
}
