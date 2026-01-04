// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.TV.SceneNumbering;

public interface ISceneMappingService
{
    Task<(int seasonNumber, int episodeNumber)?> MapSceneToTvdbAsync(int tvdbId, int sceneSeasonNumber, int sceneEpisodeNumber, CancellationToken ct = default);
    Task<(int sceneSeasonNumber, int sceneEpisodeNumber)?> MapTvdbToSceneAsync(int tvdbId, int seasonNumber, int episodeNumber, CancellationToken ct = default);

    (int seasonNumber, int episodeNumber)? MapSceneToTvdb(int tvdbId, int sceneSeasonNumber, int sceneEpisodeNumber);
    (int sceneSeasonNumber, int sceneEpisodeNumber)? MapTvdbToScene(int tvdbId, int seasonNumber, int episodeNumber);
}

public class SceneMappingService : ISceneMappingService
{
    private readonly ISceneMappingRepository _sceneMappingRepository;
    private readonly ILogger<SceneMappingService> _logger;

    public SceneMappingService(
        ISceneMappingRepository sceneMappingRepository,
        ILogger<SceneMappingService> logger)
    {
        _sceneMappingRepository = sceneMappingRepository;
        _logger = logger;
    }

    public async Task<(int seasonNumber, int episodeNumber)?> MapSceneToTvdbAsync(int tvdbId, int sceneSeasonNumber, int sceneEpisodeNumber, CancellationToken ct = default)
    {
        var mappings = await _sceneMappingRepository.GetByTvdbIdAsync(tvdbId, ct).ConfigureAwait(false);
        var mapping = mappings.FirstOrDefault(m =>
            m.SceneSeasonNumber == sceneSeasonNumber &&
            m.SceneEpisodeNumber == sceneEpisodeNumber);

        if (mapping != null && mapping.SeasonNumber.HasValue && mapping.EpisodeNumber.HasValue)
        {
            _logger.LogDebug("Mapped scene S{SceneSeason:00}E{SceneEpisode:00} to TVDB S{Season:00}E{Episode:00} for TVDB ID {TvdbId}",
                sceneSeasonNumber, sceneEpisodeNumber, mapping.SeasonNumber.Value, mapping.EpisodeNumber.Value, tvdbId);
            return (mapping.SeasonNumber.Value, mapping.EpisodeNumber.Value);
        }

        return null;
    }

    public (int seasonNumber, int episodeNumber)? MapSceneToTvdb(int tvdbId, int sceneSeasonNumber, int sceneEpisodeNumber)
    {
        var mappings = _sceneMappingRepository.GetByTvdbId(tvdbId);
        var mapping = mappings.FirstOrDefault(m =>
            m.SceneSeasonNumber == sceneSeasonNumber &&
            m.SceneEpisodeNumber == sceneEpisodeNumber);

        if (mapping != null && mapping.SeasonNumber.HasValue && mapping.EpisodeNumber.HasValue)
        {
            _logger.LogDebug("Mapped scene S{SceneSeason:00}E{SceneEpisode:00} to TVDB S{Season:00}E{Episode:00} for TVDB ID {TvdbId}",
                sceneSeasonNumber, sceneEpisodeNumber, mapping.SeasonNumber.Value, mapping.EpisodeNumber.Value, tvdbId);
            return (mapping.SeasonNumber.Value, mapping.EpisodeNumber.Value);
        }

        return null;
    }

    public async Task<(int sceneSeasonNumber, int sceneEpisodeNumber)?> MapTvdbToSceneAsync(int tvdbId, int seasonNumber, int episodeNumber, CancellationToken ct = default)
    {
        var mapping = await _sceneMappingRepository.FindMappingAsync(tvdbId, seasonNumber, episodeNumber, ct).ConfigureAwait(false);

        if (mapping != null && mapping.SceneSeasonNumber.HasValue && mapping.SceneEpisodeNumber.HasValue)
        {
            _logger.LogDebug("Mapped TVDB S{Season:00}E{Episode:00} to scene S{SceneSeason:00}E{SceneEpisode:00} for TVDB ID {TvdbId}",
                seasonNumber, episodeNumber, mapping.SceneSeasonNumber.Value, mapping.SceneEpisodeNumber.Value, tvdbId);
            return (mapping.SceneSeasonNumber.Value, mapping.SceneEpisodeNumber.Value);
        }

        return null;
    }

    public (int sceneSeasonNumber, int sceneEpisodeNumber)? MapTvdbToScene(int tvdbId, int seasonNumber, int episodeNumber)
    {
        var mapping = _sceneMappingRepository.FindMapping(tvdbId, seasonNumber, episodeNumber);

        if (mapping != null && mapping.SceneSeasonNumber.HasValue && mapping.SceneEpisodeNumber.HasValue)
        {
            _logger.LogDebug("Mapped TVDB S{Season:00}E{Episode:00} to scene S{SceneSeason:00}E{SceneEpisode:00} for TVDB ID {TvdbId}",
                seasonNumber, episodeNumber, mapping.SceneSeasonNumber.Value, mapping.SceneEpisodeNumber.Value, tvdbId);
            return (mapping.SceneSeasonNumber.Value, mapping.SceneEpisodeNumber.Value);
        }

        return null;
    }
}
