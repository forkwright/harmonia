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
using Mouseion.Core.Audiobooks;

namespace Mouseion.Api.Audiobooks;

[ApiController]
[Route("api/v3/audiobooks")]
[Authorize]
public class AudiobookController : ControllerBase
{
    private readonly IAudiobookRepository _audiobookRepository;
    private readonly IAddAudiobookService _addAudiobookService;
    private readonly IAudiobookStatisticsService _statisticsService;

    public AudiobookController(
        IAudiobookRepository audiobookRepository,
        IAddAudiobookService addAudiobookService,
        IAudiobookStatisticsService statisticsService)
    {
        _audiobookRepository = audiobookRepository;
        _addAudiobookService = addAudiobookService;
        _statisticsService = statisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<AudiobookResource>>> GetAudiobooks(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _audiobookRepository.CountAsync(ct).ConfigureAwait(false);
        var audiobooks = await _audiobookRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<AudiobookResource>
        {
            Items = audiobooks.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<AudiobookResource>> GetAudiobook(int id, CancellationToken ct = default)
    {
        var audiobook = await _audiobookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (audiobook == null)
        {
            return NotFound(new { error = $"Audiobook {id} not found" });
        }

        return Ok(ToResource(audiobook));
    }

    [HttpGet("author/{authorId:int}")]
    public async Task<ActionResult<List<AudiobookResource>>> GetAudiobooksByAuthor(int authorId, CancellationToken ct = default)
    {
        var audiobooks = await _audiobookRepository.GetByAuthorIdAsync(authorId, ct).ConfigureAwait(false);
        return Ok(audiobooks.Select(ToResource).ToList());
    }

    [HttpGet("series/{seriesId:int}")]
    public async Task<ActionResult<List<AudiobookResource>>> GetAudiobooksBySeries(int seriesId, CancellationToken ct = default)
    {
        var audiobooks = await _audiobookRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(audiobooks.Select(ToResource).ToList());
    }

    [HttpGet("statistics")]
    public async Task<ActionResult<AudiobookStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("statistics/author/{authorId:int}")]
    public async Task<ActionResult<AudiobookStatistics>> GetAuthorStatistics(int authorId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetAuthorStatisticsAsync(authorId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("statistics/series/{seriesId:int}")]
    public async Task<ActionResult<AudiobookStatistics>> GetSeriesStatistics(int seriesId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetSeriesStatisticsAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("statistics/narrator/{narrator}")]
    public async Task<ActionResult<AudiobookStatistics>> GetNarratorStatistics(string narrator, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetNarratorStatisticsAsync(narrator, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpPost]
    public async Task<ActionResult<AudiobookResource>> AddAudiobook([FromBody] AudiobookResource resource, CancellationToken ct = default)
    {
        try
        {
            var audiobook = ToModel(resource);
            var added = await _addAudiobookService.AddAudiobookAsync(audiobook, ct).ConfigureAwait(false);
            return CreatedAtAction(nameof(GetAudiobook), new { id = added.Id }, ToResource(added));
        }
        catch (ArgumentException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<AudiobookResource>>> AddAudiobooks([FromBody] List<AudiobookResource> resources, CancellationToken ct = default)
    {
        try
        {
            var audiobooks = resources.Select(ToModel).ToList();
            var added = await _addAudiobookService.AddAudiobooksAsync(audiobooks, ct).ConfigureAwait(false);
            return Ok(added.Select(ToResource).ToList());
        }
        catch (ArgumentException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<AudiobookResource>> UpdateAudiobook(int id, [FromBody] AudiobookResource resource, CancellationToken ct = default)
    {
        var audiobook = await _audiobookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (audiobook == null)
        {
            return NotFound(new { error = $"Audiobook {id} not found" });
        }

        audiobook.Title = resource.Title;
        audiobook.Year = resource.Year;
        audiobook.Monitored = resource.Monitored;
        audiobook.QualityProfileId = resource.QualityProfileId;
        audiobook.AuthorId = resource.AuthorId;
        audiobook.BookSeriesId = resource.BookSeriesId;
        audiobook.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();
        audiobook.Metadata = ToMetadata(resource.Metadata);

        var updated = await _audiobookRepository.UpdateAsync(audiobook, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteAudiobook(int id, CancellationToken ct = default)
    {
        var audiobook = await _audiobookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (audiobook == null)
        {
            return NotFound(new { error = $"Audiobook {id} not found" });
        }

        await _audiobookRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static AudiobookResource ToResource(Audiobook audiobook)
    {
        return new AudiobookResource
        {
            Id = audiobook.Id,
            Title = audiobook.Title,
            Year = audiobook.Year,
            Monitored = audiobook.Monitored,
            QualityProfileId = audiobook.QualityProfileId,
            Added = audiobook.Added,
            AuthorId = audiobook.AuthorId,
            BookSeriesId = audiobook.BookSeriesId,
            Tags = audiobook.Tags?.ToList(),
            Metadata = new AudiobookMetadataResource
            {
                Description = audiobook.Metadata.Description,
                ForeignAudiobookId = audiobook.Metadata.ForeignAudiobookId,
                AudnexusId = audiobook.Metadata.AudnexusId,
                AudibleId = audiobook.Metadata.AudibleId,
                Isbn = audiobook.Metadata.Isbn,
                Isbn13 = audiobook.Metadata.Isbn13,
                Asin = audiobook.Metadata.Asin,
                ReleaseDate = audiobook.Metadata.ReleaseDate,
                Publisher = audiobook.Metadata.Publisher,
                Language = audiobook.Metadata.Language,
                Genres = audiobook.Metadata.Genres,
                Narrator = audiobook.Metadata.Narrator,
                Narrators = audiobook.Metadata.Narrators,
                DurationMinutes = audiobook.Metadata.DurationMinutes,
                IsAbridged = audiobook.Metadata.IsAbridged,
                SeriesPosition = audiobook.Metadata.SeriesPosition,
                BookId = audiobook.Metadata.BookId
            }
        };
    }

    private static Audiobook ToModel(AudiobookResource resource)
    {
        return new Audiobook
        {
            Id = resource.Id,
            Title = resource.Title,
            Year = resource.Year,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Added = resource.Added,
            AuthorId = resource.AuthorId,
            BookSeriesId = resource.BookSeriesId,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>(),
            Metadata = ToMetadata(resource.Metadata)
        };
    }

    private static AudiobookMetadata ToMetadata(AudiobookMetadataResource? resource)
    {
        if (resource == null)
        {
            return new AudiobookMetadata();
        }

        return new AudiobookMetadata
        {
            Description = resource.Description,
            ForeignAudiobookId = resource.ForeignAudiobookId,
            AudnexusId = resource.AudnexusId,
            AudibleId = resource.AudibleId,
            Isbn = resource.Isbn,
            Isbn13 = resource.Isbn13,
            Asin = resource.Asin,
            ReleaseDate = resource.ReleaseDate,
            Publisher = resource.Publisher,
            Language = resource.Language,
            Genres = resource.Genres ?? new List<string>(),
            Narrator = resource.Narrator,
            Narrators = resource.Narrators ?? new List<string>(),
            DurationMinutes = resource.DurationMinutes,
            IsAbridged = resource.IsAbridged,
            SeriesPosition = resource.SeriesPosition,
            BookId = resource.BookId
        };
    }
}

public class AudiobookResource
{
    public int Id { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
    public int? AuthorId { get; set; }
    public int? BookSeriesId { get; set; }
    public List<int>? Tags { get; set; }
    public AudiobookMetadataResource Metadata { get; set; } = new();
}

public class AudiobookMetadataResource
{
    public string? Description { get; set; }
    public string? ForeignAudiobookId { get; set; }
    public string? AudnexusId { get; set; }
    public string? AudibleId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Asin { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public string? Narrator { get; set; }
    public List<string> Narrators { get; set; } = new();
    public int? DurationMinutes { get; set; }
    public bool IsAbridged { get; set; }
    public int? SeriesPosition { get; set; }
    public int? BookId { get; set; }
}
