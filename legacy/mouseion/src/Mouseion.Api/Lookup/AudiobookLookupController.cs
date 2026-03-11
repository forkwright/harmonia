// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Api.Lookup;

[ApiController]
[Route("api/v3/lookup/audiobooks")]
[Authorize]
public class AudiobookLookupController : ControllerBase
{
    private readonly IProvideAudiobookInfo _audiobookInfoProvider;

    public AudiobookLookupController(IProvideAudiobookInfo audiobookInfoProvider)
    {
        _audiobookInfoProvider = audiobookInfoProvider;
    }

    [HttpGet]
    public async Task<ActionResult<List<AudiobookLookupResource>>> Search(
        [FromQuery] string? title,
        [FromQuery] string? author,
        [FromQuery] string? narrator,
        [FromQuery] string? asin,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) &&
            string.IsNullOrWhiteSpace(narrator) && string.IsNullOrWhiteSpace(asin))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, narrator, or asin) is required" });
        }

        var tasks = new List<Task<List<Audiobook>>>();

        if (!string.IsNullOrWhiteSpace(title))
            tasks.Add(_audiobookInfoProvider.SearchByTitleAsync(title, ct));

        if (!string.IsNullOrWhiteSpace(author))
            tasks.Add(_audiobookInfoProvider.SearchByAuthorAsync(author, ct));

        if (!string.IsNullOrWhiteSpace(narrator))
            tasks.Add(_audiobookInfoProvider.SearchByNarratorAsync(narrator, ct));

        var searchResults = await Task.WhenAll(tasks).ConfigureAwait(false);
        var results = searchResults.SelectMany(x => x).Distinct();

        return Ok(results.Select(ToResource).ToList());
    }

    [HttpGet("{asin}")]
    public async Task<ActionResult<AudiobookLookupResource>> GetByAsin(string asin, CancellationToken ct = default)
    {
        var audiobook = await _audiobookInfoProvider.GetByAsinAsync(asin, ct).ConfigureAwait(false);
        if (audiobook == null)
        {
            return NotFound(new { error = $"Audiobook with ASIN {asin} not found" });
        }

        return Ok(ToResource(audiobook));
    }

    [HttpGet("narrator/{narrator}")]
    public async Task<ActionResult<List<AudiobookLookupResource>>> SearchByNarrator(string narrator, CancellationToken ct = default)
    {
        var audiobooks = await _audiobookInfoProvider.SearchByNarratorAsync(narrator, ct).ConfigureAwait(false);
        return Ok(audiobooks.Select(ToResource).ToList());
    }

    private static AudiobookLookupResource ToResource(Audiobook audiobook)
    {
        return new AudiobookLookupResource
        {
            Title = audiobook.Title,
            Year = audiobook.Year,
            ForeignAudiobookId = audiobook.Metadata.ForeignAudiobookId,
            AudnexusId = audiobook.Metadata.AudnexusId,
            AudibleId = audiobook.Metadata.AudibleId,
            Asin = audiobook.Metadata.Asin,
            Isbn = audiobook.Metadata.Isbn,
            Isbn13 = audiobook.Metadata.Isbn13,
            Description = audiobook.Metadata.Description,
            Narrator = audiobook.Metadata.Narrator,
            Narrators = audiobook.Metadata.Narrators,
            DurationMinutes = audiobook.Metadata.DurationMinutes,
            IsAbridged = audiobook.Metadata.IsAbridged,
            Publisher = audiobook.Metadata.Publisher,
            Language = audiobook.Metadata.Language,
            Genres = audiobook.Metadata.Genres,
            ReleaseDate = audiobook.Metadata.ReleaseDate
        };
    }
}
