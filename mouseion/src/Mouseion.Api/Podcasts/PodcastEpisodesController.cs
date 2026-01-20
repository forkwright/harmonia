// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Podcasts;

namespace Mouseion.Api.Podcasts;

[ApiController]
[Route("api/v3/podcasts")]
[Authorize]
public class PodcastEpisodesController : ControllerBase
{
    private readonly IPodcastEpisodeRepository _episodeRepository;

    public PodcastEpisodesController(IPodcastEpisodeRepository episodeRepository)
    {
        _episodeRepository = episodeRepository;
    }

    [HttpGet("{id:int}/episodes")]
    public async Task<ActionResult<List<PodcastEpisodeResource>>> GetEpisodes(int id, CancellationToken ct = default)
    {
        var episodes = await _episodeRepository.GetByShowIdAsync(id, ct).ConfigureAwait(false);
        return Ok(episodes.Select(ToEpisodeResource).ToList());
    }

    [HttpGet("episodes/{episodeId:int}")]
    public async Task<ActionResult<PodcastEpisodeResource>> GetEpisode(int episodeId, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(episodeId, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Podcast episode {episodeId} not found" });
        }

        return Ok(ToEpisodeResource(episode));
    }

    private static PodcastEpisodeResource ToEpisodeResource(PodcastEpisode episode)
    {
        return new PodcastEpisodeResource
        {
            Id = episode.Id,
            PodcastShowId = episode.PodcastShowId,
            Title = episode.Title,
            Description = episode.Description,
            EpisodeGuid = episode.EpisodeGuid,
            EpisodeNumber = episode.EpisodeNumber,
            SeasonNumber = episode.SeasonNumber,
            PublishDate = episode.PublishDate,
            Duration = episode.Duration,
            EnclosureUrl = episode.EnclosureUrl,
            EnclosureLength = episode.EnclosureLength,
            EnclosureType = episode.EnclosureType,
            ImageUrl = episode.ImageUrl,
            Explicit = episode.Explicit,
            Monitored = episode.Monitored,
            Added = episode.Added
        };
    }
}
