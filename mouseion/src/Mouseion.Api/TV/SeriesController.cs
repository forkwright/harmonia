// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.TV;

namespace Mouseion.Api.TV;

[ApiController]
[Route("api/v3/series")]
[Authorize]
public class SeriesController : ControllerBase
{
    private readonly ISeriesRepository _seriesRepository;
    private readonly IAddSeriesService _addSeriesService;
    private readonly ISeriesStatisticsService _seriesStatisticsService;

    public SeriesController(
        ISeriesRepository seriesRepository,
        IAddSeriesService addSeriesService,
        ISeriesStatisticsService seriesStatisticsService)
    {
        _seriesRepository = seriesRepository;
        _addSeriesService = addSeriesService;
        _seriesStatisticsService = seriesStatisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<SeriesResource>>> GetSeries(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _seriesRepository.CountAsync(ct).ConfigureAwait(false);
        var series = await _seriesRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<SeriesResource>
        {
            Items = series.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<SeriesResource>> GetSeriesById(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        return Ok(ToResource(series));
    }

    [HttpGet("tvdb/{tvdbId:int}")]
    public async Task<ActionResult<SeriesResource>> GetByTvdbId(int tvdbId, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindByTvdbIdAsync(tvdbId, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series with TVDB ID {tvdbId} not found" });
        }

        return Ok(ToResource(series));
    }

    [HttpGet("statistics")]
    public async Task<ActionResult<SeriesStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _seriesStatisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpPost]
    public async Task<ActionResult<SeriesResource>> AddSeries([FromBody] SeriesResource resource, CancellationToken ct = default)
    {
        var series = ToModel(resource);
        var added = await _addSeriesService.AddSeriesAsync(series, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetSeriesById), new { id = added.Id }, ToResource(added));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<SeriesResource>>> AddSeriesList([FromBody] List<SeriesResource> resources, CancellationToken ct = default)
    {
        var seriesList = resources.Select(ToModel).ToList();
        var added = await _addSeriesService.AddSeriesListAsync(seriesList, ct).ConfigureAwait(false);
        return Ok(added.Select(ToResource).ToList());
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<SeriesResource>> UpdateSeries(int id, [FromBody] SeriesResource resource, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        series.Title = resource.Title;
        series.Year = resource.Year;
        series.Overview = resource.Overview;
        series.Status = resource.Status;
        series.AirTime = resource.AirTime;
        series.Network = resource.Network;
        series.Runtime = resource.Runtime;
        series.Genres = resource.Genres ?? new List<string>();
        series.FirstAired = resource.FirstAired;
        series.Images = resource.Images ?? new List<string>();
        series.TvdbId = resource.TvdbId;
        series.TmdbId = resource.TmdbId;
        series.ImdbId = resource.ImdbId;
        series.Path = resource.Path;
        series.RootFolderPath = resource.RootFolderPath;
        series.QualityProfileId = resource.QualityProfileId;
        series.SeasonFolder = resource.SeasonFolder;
        series.Monitored = resource.Monitored;
        series.UseSceneNumbering = resource.UseSceneNumbering;
        series.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();

        var updated = await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        await _seriesRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static SeriesResource ToResource(Series series)
    {
        return new SeriesResource
        {
            Id = series.Id,
            Title = series.Title,
            Year = series.Year,
            Overview = series.Overview,
            Status = series.Status,
            AirTime = series.AirTime,
            Network = series.Network,
            Runtime = series.Runtime,
            Genres = series.Genres,
            FirstAired = series.FirstAired,
            Images = series.Images,
            TvdbId = series.TvdbId,
            TmdbId = series.TmdbId,
            ImdbId = series.ImdbId,
            Path = series.Path,
            RootFolderPath = series.RootFolderPath,
            QualityProfileId = series.QualityProfileId,
            SeasonFolder = series.SeasonFolder,
            Monitored = series.Monitored,
            UseSceneNumbering = series.UseSceneNumbering,
            Added = series.Added,
            Tags = series.Tags?.ToList()
        };
    }

    private static Series ToModel(SeriesResource resource)
    {
        return new Series
        {
            Id = resource.Id,
            Title = resource.Title,
            Year = resource.Year,
            Overview = resource.Overview,
            Status = resource.Status,
            AirTime = resource.AirTime,
            Network = resource.Network,
            Runtime = resource.Runtime,
            Genres = resource.Genres ?? new List<string>(),
            FirstAired = resource.FirstAired,
            Images = resource.Images ?? new List<string>(),
            TvdbId = resource.TvdbId,
            TmdbId = resource.TmdbId,
            ImdbId = resource.ImdbId,
            Path = resource.Path,
            RootFolderPath = resource.RootFolderPath,
            QualityProfileId = resource.QualityProfileId,
            SeasonFolder = resource.SeasonFolder,
            Monitored = resource.Monitored,
            UseSceneNumbering = resource.UseSceneNumbering,
            Added = resource.Added,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>()
        };
    }
}
