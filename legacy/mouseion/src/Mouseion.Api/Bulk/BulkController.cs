// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Bulk;

namespace Mouseion.Api.Bulk;

[ApiController]
[Route("api/v3/bulk")]
[Authorize]
public class BulkController : ControllerBase
{
    private readonly IBulkOperationsService _bulkOperationsService;

    public BulkController(IBulkOperationsService bulkOperationsService)
    {
        _bulkOperationsService = bulkOperationsService;
    }

    [HttpPost("movies/update")]
    public async Task<ActionResult<BulkUpdateResult>> UpdateMovies(
        [FromBody] BulkUpdateRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.UpdateMoviesAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("movies/delete")]
    public async Task<ActionResult<BulkDeleteResult>> DeleteMovies(
        [FromBody] BulkDeleteRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.DeleteMoviesAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("books/update")]
    public async Task<ActionResult<BulkUpdateResult>> UpdateBooks(
        [FromBody] BulkUpdateRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.UpdateBooksAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("books/delete")]
    public async Task<ActionResult<BulkDeleteResult>> DeleteBooks(
        [FromBody] BulkDeleteRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.DeleteBooksAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("audiobooks/update")]
    public async Task<ActionResult<BulkUpdateResult>> UpdateAudiobooks(
        [FromBody] BulkUpdateRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.UpdateAudiobooksAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("audiobooks/delete")]
    public async Task<ActionResult<BulkDeleteResult>> DeleteAudiobooks(
        [FromBody] BulkDeleteRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.DeleteAudiobooksAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("series/update")]
    public async Task<ActionResult<BulkUpdateResult>> UpdateSeries(
        [FromBody] BulkUpdateRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.UpdateSeriesAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("series/delete")]
    public async Task<ActionResult<BulkDeleteResult>> DeleteSeries(
        [FromBody] BulkDeleteRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.DeleteSeriesAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("manga/chapters/read")]
    public async Task<ActionResult<BulkReadResult>> MarkMangaChaptersRead(
        [FromBody] BulkReadRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.MarkMangaChaptersReadAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("webcomic/episodes/read")]
    public async Task<ActionResult<BulkReadResult>> MarkWebcomicEpisodesRead(
        [FromBody] BulkReadRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.MarkWebcomicEpisodesReadAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("comic/issues/read")]
    public async Task<ActionResult<BulkReadResult>> MarkComicIssuesRead(
        [FromBody] BulkReadRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.MarkComicIssuesReadAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }

    [HttpPost("articles/read")]
    public async Task<ActionResult<BulkReadResult>> MarkArticlesRead(
        [FromBody] BulkReadRequest request,
        CancellationToken ct = default)
    {
        var result = await _bulkOperationsService.MarkArticlesReadAsync(request, ct).ConfigureAwait(false);
        return Ok(result);
    }
}
