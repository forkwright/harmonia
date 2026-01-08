// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.TV;

namespace Mouseion.Api.TV;

[ApiController]
[Route("api/v3/episodes")]
[Authorize]
public class EpisodeController : ControllerBase
{
    private readonly IEpisodeRepository _episodeRepository;

    public EpisodeController(IEpisodeRepository episodeRepository)
    {
        _episodeRepository = episodeRepository;
    }

    [HttpGet("series/{seriesId:int}")]
    public async Task<ActionResult<List<EpisodeResource>>> GetEpisodesBySeries(int seriesId, CancellationToken ct = default)
    {
        var episodes = await _episodeRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(episodes.Select(ToResource).ToList());
    }

    [HttpGet("series/{seriesId:int}/season/{seasonNumber:int}")]
    public async Task<ActionResult<List<EpisodeResource>>> GetEpisodesBySeason(int seriesId, int seasonNumber, CancellationToken ct = default)
    {
        var episodes = await _episodeRepository.GetBySeasonAsync(seriesId, seasonNumber, ct).ConfigureAwait(false);
        return Ok(episodes.Select(ToResource).ToList());
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<EpisodeResource>> GetEpisode(int id, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Episode {id} not found" });
        }

        return Ok(ToResource(episode));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<EpisodeResource>> UpdateEpisode(int id, [FromBody][Required] EpisodeResource resource, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Episode {id} not found" });
        }

        episode.Title = resource.Title;
        episode.Overview = resource.Overview;
        episode.SeasonNumber = resource.SeasonNumber;
        episode.EpisodeNumber = resource.EpisodeNumber;
        episode.AbsoluteEpisodeNumber = resource.AbsoluteEpisodeNumber;
        episode.SceneSeasonNumber = resource.SceneSeasonNumber;
        episode.SceneEpisodeNumber = resource.SceneEpisodeNumber;
        episode.SceneAbsoluteEpisodeNumber = resource.SceneAbsoluteEpisodeNumber;
        episode.AirDate = resource.AirDate;
        episode.AirDateUtc = resource.AirDateUtc;
        episode.Monitored = resource.Monitored;

        var updated = await _episodeRepository.UpdateAsync(episode, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    private static EpisodeResource ToResource(Episode episode)
    {
        return new EpisodeResource
        {
            Id = episode.Id,
            SeriesId = episode.SeriesId,
            SeasonNumber = episode.SeasonNumber,
            EpisodeNumber = episode.EpisodeNumber,
            Title = episode.Title,
            Overview = episode.Overview,
            AirDate = episode.AirDate,
            AirDateUtc = episode.AirDateUtc,
            EpisodeFileId = episode.EpisodeFileId,
            AbsoluteEpisodeNumber = episode.AbsoluteEpisodeNumber,
            SceneSeasonNumber = episode.SceneSeasonNumber,
            SceneEpisodeNumber = episode.SceneEpisodeNumber,
            SceneAbsoluteEpisodeNumber = episode.SceneAbsoluteEpisodeNumber,
            Monitored = episode.Monitored
        };
    }
}
