// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.Indexers.MyAnonamouse;

namespace Mouseion.Api.Lookup;

[ApiController]
[Route("api/v3/lookup/torrents")]
[Authorize]
public class TorrentLookupController : ControllerBase
{
    private readonly IMyAnonamouseIndexer _myAnonamouseIndexer;

    public TorrentLookupController(IMyAnonamouseIndexer myAnonamouseIndexer)
    {
        _myAnonamouseIndexer = myAnonamouseIndexer;
    }

    [HttpGet("books")]
    public async Task<ActionResult<List<TorrentLookupResource>>> SearchBooks(
        [FromQuery] string? title,
        [FromQuery] string? author,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author))
        {
            return BadRequest(new { error = "At least one search parameter (title or author) is required" });
        }

        var criteria = new BookSearchCriteria
        {
            Title = title,
            Author = author
        };

        var results = await _myAnonamouseIndexer.SearchBooksAsync(criteria, ct).ConfigureAwait(false);
        return Ok(results.Select(ToResource).ToList());
    }

    [HttpGet("audiobooks")]
    public async Task<ActionResult<List<TorrentLookupResource>>> SearchAudiobooks(
        [FromQuery] string? title,
        [FromQuery] string? author,
        [FromQuery] string? narrator,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) && string.IsNullOrWhiteSpace(narrator))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, or narrator) is required" });
        }

        var criteria = new AudiobookSearchCriteria
        {
            Title = title,
            Author = author,
            Narrator = narrator
        };

        var results = await _myAnonamouseIndexer.SearchAudiobooksAsync(criteria, ct).ConfigureAwait(false);
        return Ok(results.Select(ToResource).ToList());
    }

    private static TorrentLookupResource ToResource(IndexerResult result)
    {
        return new TorrentLookupResource
        {
            TorrentId = result.TorrentId,
            Title = result.Title,
            Author = result.Author,
            Category = result.Category,
            Size = result.Size,
            Seeders = result.Seeders,
            Leechers = result.Leechers,
            PublishDate = result.PublishDate,
            DownloadUrl = result.DownloadUrl,
            InfoUrl = result.InfoUrl,
            IsFreeleech = result.IsFreeleech
        };
    }
}
