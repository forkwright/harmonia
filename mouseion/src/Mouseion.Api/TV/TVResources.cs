// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.TV;

public class SeriesResource
{
    public int Id { get; set; }
    public int? TvdbId { get; set; }
    public int? TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? Overview { get; set; }
    public string Status { get; set; } = "Continuing";
    public string? AirTime { get; set; }
    public string? Network { get; set; }
    public int? Runtime { get; set; }
    public List<string>? Genres { get; set; }
    public DateTime? FirstAired { get; set; }
    public List<string>? Images { get; set; }
    public string Path { get; set; } = null!;
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public bool SeasonFolder { get; set; } = true;
    public bool Monitored { get; set; }
    public bool UseSceneNumbering { get; set; }
    public DateTime Added { get; set; }
    public List<int>? Tags { get; set; }
}

public class SeasonResource
{
    public int Id { get; set; }
    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public bool Monitored { get; set; }
}

public class EpisodeResource
{
    public int Id { get; set; }
    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public int EpisodeNumber { get; set; }
    public string? Title { get; set; }
    public string? Overview { get; set; }
    public DateTime? AirDate { get; set; }
    public DateTime? AirDateUtc { get; set; }
    public int? EpisodeFileId { get; set; }
    public int? AbsoluteEpisodeNumber { get; set; }
    public int? SceneSeasonNumber { get; set; }
    public int? SceneEpisodeNumber { get; set; }
    public int? SceneAbsoluteEpisodeNumber { get; set; }
    public bool Monitored { get; set; }
}

public class EpisodeFileResource
{
    public int Id { get; set; }
    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public string RelativePath { get; set; } = null!;
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public string? SceneName { get; set; }
    public string? ReleaseGroup { get; set; }
}
