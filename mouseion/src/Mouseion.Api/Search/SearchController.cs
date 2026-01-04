// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Music;

namespace Mouseion.Api.Search;

[ApiController]
[Route("api/v3/search")]
[Authorize]
public class SearchController : ControllerBase
{
    private readonly ITrackSearchService _trackSearchService;

    public SearchController(ITrackSearchService trackSearchService)
    {
        _trackSearchService = trackSearchService;
    }

    [HttpGet]
    public async Task<ActionResult<List<TrackSearchResource>>> Search(
        [FromQuery] string? q,
        [FromQuery] int limit = 50,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(q))
        {
            return BadRequest(new { error = "Query parameter 'q' is required" });
        }

        if (limit < 1) limit = 50;
        if (limit > 250) limit = 250;

        var results = await _trackSearchService.SearchAsync(q, limit, ct).ConfigureAwait(false);
        return Ok(results.Select(ToResource).ToList());
    }

    private static TrackSearchResource ToResource(TrackSearchResult result)
    {
        return new TrackSearchResource
        {
            TrackId = result.Track.Id,
            Title = result.Track.Title,
            Artist = result.ArtistName,
            Album = result.AlbumTitle,
            TrackNumber = result.Track.TrackNumber,
            DiscNumber = result.Track.DiscNumber,
            DurationSeconds = result.Track.DurationSeconds,
            Genre = result.Genre,
            BitDepth = result.BitDepth,
            DynamicRange = result.DynamicRange,
            Lossless = result.Lossless,
            RelevanceScore = result.RelevanceScore
        };
    }
}

public class TrackSearchResource
{
    public int TrackId { get; set; }
    public string Title { get; set; } = null!;
    public string? Artist { get; set; }
    public string? Album { get; set; }
    public int TrackNumber { get; set; }
    public int DiscNumber { get; set; }
    public int? DurationSeconds { get; set; }
    public string? Genre { get; set; }
    public int? BitDepth { get; set; }
    public int? DynamicRange { get; set; }
    public bool Lossless { get; set; }
    public double RelevanceScore { get; set; }
}
