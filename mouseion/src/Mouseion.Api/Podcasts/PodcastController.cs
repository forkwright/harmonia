// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Podcasts;

namespace Mouseion.Api.Podcasts;

[ApiController]
[Route("api/v3/podcasts")]
[Authorize]
public class PodcastController : ControllerBase
{
    private readonly IPodcastShowRepository _showRepository;
    private readonly IPodcastEpisodeRepository _episodeRepository;
    private readonly IAddPodcastService _addPodcastService;

    public PodcastController(
        IPodcastShowRepository showRepository,
        IPodcastEpisodeRepository episodeRepository,
        IAddPodcastService addPodcastService)
    {
        _showRepository = showRepository;
        _episodeRepository = episodeRepository;
        _addPodcastService = addPodcastService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<PodcastShowResource>>> GetPodcasts(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _showRepository.CountAsync(ct).ConfigureAwait(false);
        var shows = await _showRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<PodcastShowResource>
        {
            Items = shows.Select(ToShowResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<PodcastShowResource>> GetPodcast(int id, CancellationToken ct = default)
    {
        var show = await _showRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (show == null)
        {
            return NotFound(new { error = $"Podcast show {id} not found" });
        }

        return Ok(ToShowResource(show));
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

    [HttpPost]
    public async Task<ActionResult<PodcastShowResource>> AddPodcast([FromBody][Required] AddPodcastRequest request, CancellationToken ct = default)
    {
        var show = await _addPodcastService.AddPodcastAsync(
        request.FeedUrl,
        request.RootFolderPath,
        request.QualityProfileId,
        request.Monitored,
        ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(GetPodcast), new { id = show.Id }, ToShowResource(show));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<PodcastShowResource>> UpdatePodcast(int id, [FromBody][Required] PodcastShowResource resource, CancellationToken ct = default)
    {
        var show = await _showRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (show == null)
        {
            return NotFound(new { error = $"Podcast show {id} not found" });
        }

        show.Title = resource.Title;
        show.Monitored = resource.Monitored;
        show.MonitorNewEpisodes = resource.MonitorNewEpisodes;
        show.QualityProfileId = resource.QualityProfileId;
        show.Path = resource.Path;
        show.RootFolderPath = resource.RootFolderPath;

        var updated = await _showRepository.UpdateAsync(show, ct).ConfigureAwait(false);
        return Ok(ToShowResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeletePodcast(int id, CancellationToken ct = default)
    {
        var show = await _showRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (show == null)
        {
            return NotFound(new { error = $"Podcast show {id} not found" });
        }

        await _showRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static PodcastShowResource ToShowResource(PodcastShow show)
    {
        return new PodcastShowResource
        {
            Id = show.Id,
            Title = show.Title,
            SortTitle = show.SortTitle,
            Description = show.Description,
            ForeignPodcastId = show.ForeignPodcastId,
            ItunesId = show.ItunesId,
            Author = show.Author,
            FeedUrl = show.FeedUrl,
            ImageUrl = show.ImageUrl,
            Categories = show.Categories,
            Language = show.Language,
            Website = show.Website,
            EpisodeCount = show.EpisodeCount,
            LatestEpisodeDate = show.LatestEpisodeDate,
            Monitored = show.Monitored,
            MonitorNewEpisodes = show.MonitorNewEpisodes,
            Path = show.Path,
            RootFolderPath = show.RootFolderPath,
            QualityProfileId = show.QualityProfileId,
            Tags = show.Tags,
            Added = show.Added,
            LastSearchTime = show.LastSearchTime
        };
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

public class PodcastShowResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignPodcastId { get; set; }
    public string? ItunesId { get; set; }
    public string? Author { get; set; }
    public string FeedUrl { get; set; } = string.Empty;
    public string? ImageUrl { get; set; }
    public string? Categories { get; set; }
    public string? Language { get; set; }
    public string? Website { get; set; }
    public int? EpisodeCount { get; set; }
    public DateTime? LatestEpisodeDate { get; set; }
    public bool Monitored { get; set; }
    public bool MonitorNewEpisodes { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public string? Tags { get; set; }
    public DateTime Added { get; set; }
    public DateTime? LastSearchTime { get; set; }
}

public class PodcastEpisodeResource
{
    public int Id { get; set; }
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
    public bool Monitored { get; set; }
    public DateTime Added { get; set; }
}

public class AddPodcastRequest
{
    public string FeedUrl { get; set; } = string.Empty;
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public bool Monitored { get; set; } = true;
}
