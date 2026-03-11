// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webcomic;

public class WebcomicEpisode : ModelBase
{
    public int WebcomicSeriesId { get; set; }
    public string? Title { get; set; }
    public int? EpisodeNumber { get; set; }
    public int? SeasonNumber { get; set; }
    public string? ExternalId { get; set; }
    public string? ExternalUrl { get; set; }
    public string? ThumbnailUrl { get; set; }
    public DateTime? PublishDate { get; set; }
    public bool IsRead { get; set; }
    public bool IsDownloaded { get; set; }
    public string? FilePath { get; set; }
    public DateTime Added { get; set; }

    public override string ToString() => Title ?? $"Episode {EpisodeNumber}";
}
