// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Webcomic;

namespace Mouseion.Api.Webcomic;

[ApiController]
[Route("api/v3/webcomic")]
[Authorize]
public class WebcomicSeriesController : ControllerBase
{
    private readonly IWebcomicSeriesRepository _seriesRepository;
    private readonly IWebcomicEpisodeRepository _episodeRepository;
    private readonly IAddWebcomicSeriesService _addSeriesService;

    public WebcomicSeriesController(
        IWebcomicSeriesRepository seriesRepository,
        IWebcomicEpisodeRepository episodeRepository,
        IAddWebcomicSeriesService addSeriesService)
    {
        _seriesRepository = seriesRepository;
        _episodeRepository = episodeRepository;
        _addSeriesService = addSeriesService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<WebcomicSeriesResource>>> GetSeries(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _seriesRepository.CountAsync(ct).ConfigureAwait(false);
        var series = await _seriesRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        var resources = new List<WebcomicSeriesResource>();
        foreach (var s in series)
        {
            var resource = ToSeriesResource(s);
            resource.UnreadEpisodeCount = await _episodeRepository.GetUnreadCountBySeriesAsync(s.Id, ct).ConfigureAwait(false);
            resources.Add(resource);
        }

        return Ok(new PagedResult<WebcomicSeriesResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<WebcomicSeriesResource>> GetSeriesById(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Webcomic series {id} not found" });
        }

        var resource = ToSeriesResource(series);
        resource.UnreadEpisodeCount = await _episodeRepository.GetUnreadCountBySeriesAsync(id, ct).ConfigureAwait(false);
        return Ok(resource);
    }

    [HttpGet("{id:int}/episodes")]
    public async Task<ActionResult<PagedResult<WebcomicEpisodeResource>>> GetSeriesEpisodes(
        int id,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        [FromQuery] bool? unreadOnly = null,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Webcomic series {id} not found" });
        }

        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var episodes = await _episodeRepository.GetBySeriesIdAsync(id, ct).ConfigureAwait(false);
        if (unreadOnly == true)
        {
            episodes = episodes.Where(e => !e.IsRead).ToList();
        }

        var totalCount = episodes.Count;
        var pagedEpisodes = episodes.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<WebcomicEpisodeResource>
        {
            Items = pagedEpisodes.Select(ToEpisodeResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpPost]
    public async Task<ActionResult<WebcomicSeriesResource>> AddSeries(
        [FromBody][Required] AddWebcomicSeriesRequest request,
        CancellationToken ct = default)
    {
        var series = new WebcomicSeries
        {
            Title = request.Title,
            Description = request.Description,
            WebtoonId = request.WebtoonId,
            TapasId = request.TapasId,
            Author = request.Author,
            Artist = request.Artist,
            Platform = request.Platform,
            SiteUrl = request.SiteUrl,
            CoverUrl = request.CoverUrl,
            RootFolderPath = request.RootFolderPath,
            QualityProfileId = request.QualityProfileId,
            Monitored = request.Monitored
        };

        var insertedSeries = await _addSeriesService.AddSeriesAsync(series, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetSeriesById), new { id = insertedSeries.Id }, ToSeriesResource(insertedSeries));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<WebcomicSeriesResource>> UpdateSeries(
        int id,
        [FromBody][Required] WebcomicSeriesResource resource,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Webcomic series {id} not found" });
        }

        series.Title = resource.Title;
        series.Description = resource.Description;
        series.Monitored = resource.Monitored;
        series.QualityProfileId = resource.QualityProfileId;
        series.Path = resource.Path;
        series.RootFolderPath = resource.RootFolderPath;

        var updated = await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
        return Ok(ToSeriesResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Webcomic series {id} not found" });
        }

        await _seriesRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/markallread")]
    public async Task<IActionResult> MarkAllRead(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Webcomic series {id} not found" });
        }

        await _episodeRepository.MarkAllReadBySeriesAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static WebcomicSeriesResource ToSeriesResource(WebcomicSeries series)
    {
        return new WebcomicSeriesResource
        {
            Id = series.Id,
            Title = series.Title,
            SortTitle = series.SortTitle,
            Description = series.Description,
            WebtoonId = series.WebtoonId,
            TapasId = series.TapasId,
            Author = series.Author,
            Artist = series.Artist,
            Status = series.Status,
            Platform = series.Platform,
            UpdateSchedule = series.UpdateSchedule,
            Genres = series.Genres,
            Tags = series.Tags,
            CoverUrl = series.CoverUrl,
            ThumbnailUrl = series.ThumbnailUrl,
            SiteUrl = series.SiteUrl,
            LastEpisodeNumber = series.LastEpisodeNumber,
            EpisodeCount = series.EpisodeCount,
            Monitored = series.Monitored,
            Path = series.Path,
            RootFolderPath = series.RootFolderPath,
            QualityProfileId = series.QualityProfileId,
            Added = series.Added
        };
    }

    private static WebcomicEpisodeResource ToEpisodeResource(WebcomicEpisode episode)
    {
        return new WebcomicEpisodeResource
        {
            Id = episode.Id,
            WebcomicSeriesId = episode.WebcomicSeriesId,
            Title = episode.Title,
            EpisodeNumber = episode.EpisodeNumber,
            SeasonNumber = episode.SeasonNumber,
            ExternalId = episode.ExternalId,
            ExternalUrl = episode.ExternalUrl,
            ThumbnailUrl = episode.ThumbnailUrl,
            PublishDate = episode.PublishDate,
            IsRead = episode.IsRead,
            IsDownloaded = episode.IsDownloaded,
            Added = episode.Added
        };
    }
}

public class WebcomicSeriesResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? WebtoonId { get; set; }
    public string? TapasId { get; set; }
    public string? Author { get; set; }
    public string? Artist { get; set; }
    public string? Status { get; set; }
    public string? Platform { get; set; }
    public string? UpdateSchedule { get; set; }
    public string? Genres { get; set; }
    public string? Tags { get; set; }
    public string? CoverUrl { get; set; }
    public string? ThumbnailUrl { get; set; }
    public string? SiteUrl { get; set; }
    public int? LastEpisodeNumber { get; set; }
    public int? EpisodeCount { get; set; }
    public int UnreadEpisodeCount { get; set; }
    public bool Monitored { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
}

public class WebcomicEpisodeResource
{
    public int Id { get; set; }
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
    public DateTime Added { get; set; }
}

public class AddWebcomicSeriesRequest
{
    public string Title { get; set; } = string.Empty;
    public string? Description { get; set; }
    public string? WebtoonId { get; set; }
    public string? TapasId { get; set; }
    public string? Author { get; set; }
    public string? Artist { get; set; }
    public string? Platform { get; set; }
    public string? SiteUrl { get; set; }
    public string? CoverUrl { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public bool Monitored { get; set; } = true;
}
