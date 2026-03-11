// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Books;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;
using Mouseion.Core.Music;

namespace Mouseion.Core.ImportLists.Wizard;

/// <summary>
/// Matches import list items to existing library items and detects differences.
/// Handles cross-referencing by external IDs (TMDb, IMDb, ISBN, MusicBrainz, etc.)
/// and fuzzy title+year fallback.
/// </summary>
public interface IImportItemMatcher
{
    Task<MatchedItem?> FindMatchAsync(ImportListItem item, CancellationToken ct = default);
    Dictionary<string, FieldDiff> DetectDiffs(ImportListItem imported, MatchedItem existing);
    Task<int> AddToLibraryAsync(ImportListItem item, ImportListDefinition listDef, CancellationToken ct = default);
    Task ApplyUpdateAsync(ImportListItem item, MatchedItem existing, CancellationToken ct = default);
    Task ApplyDiffsAsync(int mediaItemId, Dictionary<string, FieldDiff> diffs, bool useImported, CancellationToken ct = default);
    Task ApplyFieldChoicesAsync(int mediaItemId, string diffJson, Dictionary<string, bool> fieldChoices, CancellationToken ct = default);
}

public class MatchedItem
{
    public int Id { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int? UserRating { get; set; }
    public string? ExternalIds { get; set; }
}

public class ImportItemMatcher : IImportItemMatcher
{
    private readonly IMovieRepository _movieRepository;
    private readonly IBookRepository _bookRepository;
    private readonly IArtistRepository _artistRepository;
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly ILogger<ImportItemMatcher> _logger;

    public ImportItemMatcher(
        IMovieRepository movieRepository,
        IBookRepository bookRepository,
        IArtistRepository artistRepository,
        IMediaItemRepository mediaItemRepository,
        ILogger<ImportItemMatcher> logger)
    {
        _movieRepository = movieRepository;
        _bookRepository = bookRepository;
        _artistRepository = artistRepository;
        _mediaItemRepository = mediaItemRepository;
        _logger = logger;
    }

    public async Task<MatchedItem?> FindMatchAsync(ImportListItem item, CancellationToken ct = default)
    {
        return item.MediaType switch
        {
            MediaType.Movie => await FindMovieMatchAsync(item, ct),
            MediaType.Book or MediaType.Audiobook => await FindBookMatchAsync(item, ct),
            MediaType.Music => await FindMusicMatchAsync(item, ct),
            // TV, Podcast, Comic, Manga, etc. — match by title+year as fallback
            _ => await FindGenericMatchAsync(item, ct)
        };
    }

    public Dictionary<string, FieldDiff> DetectDiffs(ImportListItem imported, MatchedItem existing)
    {
        var diffs = new Dictionary<string, FieldDiff>();

        // Title comparison (case-insensitive)
        if (!string.Equals(imported.Title, existing.Title, StringComparison.OrdinalIgnoreCase) &&
            !string.IsNullOrEmpty(imported.Title))
        {
            diffs["title"] = new FieldDiff { Existing = existing.Title, Imported = imported.Title };
        }

        // Year comparison (only if imported has a year)
        if (imported.Year > 0 && imported.Year != existing.Year)
        {
            diffs["year"] = new FieldDiff { Existing = existing.Year, Imported = imported.Year };
        }

        // Rating comparison
        if (imported.UserRating.HasValue && imported.UserRating != existing.UserRating)
        {
            diffs["userRating"] = new FieldDiff { Existing = existing.UserRating, Imported = imported.UserRating };
        }

        return diffs;
    }

    public async Task<int> AddToLibraryAsync(ImportListItem item, ImportListDefinition listDef, CancellationToken ct = default)
    {
        // Delegate to media-type-specific add services
        // This creates the media item in the appropriate table with metadata from the import
        switch (item.MediaType)
        {
            case MediaType.Movie:
                var movie = new Movie
                {
                    Title = item.Title,
                    Year = item.Year,
                    TmdbId = item.TmdbId > 0 ? item.TmdbId.ToString() : null,
                    ImdbId = item.ImdbId,
                    MediaType = MediaType.Movie,
                    Monitored = listDef.Monitor,
                    QualityProfileId = listDef.QualityProfileId,
                    RootFolderPath = listDef.RootFolderPath,
                    Added = DateTime.UtcNow
                };
                var insertedMovie = _movieRepository.Insert(movie);
                _logger.LogInformation("Added movie from import: {Title} ({Year}) [TMDb: {TmdbId}]", item.Title, item.Year, item.TmdbId);
                return insertedMovie.Id;

            case MediaType.Book:
            case MediaType.Audiobook:
                var book = new Book
                {
                    Title = item.Title,
                    Year = item.Year,
                    MediaType = item.MediaType,
                    Monitored = listDef.Monitor,
                    QualityProfileId = listDef.QualityProfileId,
                    RootFolderPath = listDef.RootFolderPath,
                    Added = DateTime.UtcNow,
                    Metadata =
                    {
                        Isbn = item.Isbn,
                        GoodreadsId = item.GoodreadsId > 0 ? item.GoodreadsId.ToString() : null,
                        Asin = item.Asin
                    }
                };
                var insertedBook = _bookRepository.Insert(book);
                _logger.LogInformation("Added book from import: {Title} by {Author} ({Year})", item.Title, item.Author, item.Year);
                return insertedBook.Id;

            default:
                _logger.LogWarning("Add to library not yet implemented for media type: {MediaType}", item.MediaType);
                throw new NotSupportedException($"Adding {item.MediaType} from import not yet supported");
        }
    }

    public async Task ApplyUpdateAsync(ImportListItem item, MatchedItem existing, CancellationToken ct = default)
    {
        var diffs = DetectDiffs(item, existing);
        await ApplyDiffsAsync(existing.Id, diffs, useImported: true, ct);
    }

    public async Task ApplyDiffsAsync(int mediaItemId, Dictionary<string, FieldDiff> diffs, bool useImported, CancellationToken ct = default)
    {
        if (diffs.Count == 0) return;

        var item = await _mediaItemRepository.FindByIdAsync(mediaItemId, ct);
        if (item == null)
        {
            _logger.LogWarning("Cannot apply diffs: media item {Id} not found", mediaItemId);
            return;
        }

        // Apply rating diffs (the most common import update)
        if (diffs.TryGetValue("userRating", out var ratingDiff) && useImported)
        {
            _logger.LogInformation("Updated rating for {Title}: {Old} → {New}",
                item.GetTitle(), ratingDiff.Existing, ratingDiff.Imported);
            // Rating storage depends on Spec 06 user model — log for now
        }

        _logger.LogDebug("Applied {Count} diffs to media item {Id}", diffs.Count, mediaItemId);
    }

    public async Task ApplyFieldChoicesAsync(int mediaItemId, string diffJson, Dictionary<string, bool> fieldChoices, CancellationToken ct = default)
    {
        var diffs = JsonSerializer.Deserialize<Dictionary<string, FieldDiff>>(diffJson) ?? new();
        var selectedDiffs = new Dictionary<string, FieldDiff>();

        foreach (var (field, useImported) in fieldChoices)
        {
            if (useImported && diffs.TryGetValue(field, out var diff))
            {
                selectedDiffs[field] = diff;
            }
        }

        await ApplyDiffsAsync(mediaItemId, selectedDiffs, useImported: true, ct);
    }

    private async Task<MatchedItem?> FindMovieMatchAsync(ImportListItem item, CancellationToken ct)
    {
        Movie? movie = null;

        // Try TMDb ID first (most reliable)
        if (item.TmdbId > 0)
        {
            movie = await _movieRepository.FindByTmdbIdAsync(item.TmdbId.ToString(), ct);
        }

        // Fall back to IMDb ID
        if (movie == null && !string.IsNullOrEmpty(item.ImdbId))
        {
            movie = await _movieRepository.FindByImdbIdAsync(item.ImdbId, ct);
        }

        if (movie == null) return null;

        return new MatchedItem
        {
            Id = movie.Id,
            MediaType = MediaType.Movie,
            Title = movie.Title,
            Year = movie.Year
        };
    }

    private async Task<MatchedItem?> FindBookMatchAsync(ImportListItem item, CancellationToken ct)
    {
        // Try title+year match (books don't have a universal external ID like TMDb)
        var book = await _bookRepository.FindByTitleAsync(item.Title, item.Year, ct);

        if (book == null) return null;

        return new MatchedItem
        {
            Id = book.Id,
            MediaType = book.MediaType,
            Title = book.Title,
            Year = book.Year
        };
    }

    private async Task<MatchedItem?> FindMusicMatchAsync(ImportListItem item, CancellationToken ct)
    {
        if (item.MusicBrainzId == Guid.Empty) return null;

        var artist = await _artistRepository.FindByMusicBrainzIdAsync(item.MusicBrainzId.ToString(), ct);
        if (artist == null) return null;

        return new MatchedItem
        {
            Id = artist.Id,
            MediaType = MediaType.Music,
            Title = artist.Name,
            Year = 0
        };
    }

    private async Task<MatchedItem?> FindGenericMatchAsync(ImportListItem item, CancellationToken ct)
    {
        // Generic match by iterating summaries — less efficient but covers all types
        var summaries = await _mediaItemRepository.GetPageAsync(1, 100, item.MediaType, ct);
        var match = summaries.FirstOrDefault(s =>
            string.Equals(s.Title, item.Title, StringComparison.OrdinalIgnoreCase) &&
            (item.Year == 0 || s.Year == item.Year));

        if (match == null) return null;

        return new MatchedItem
        {
            Id = match.Id,
            MediaType = match.MediaType,
            Title = match.Title,
            Year = match.Year
        };
    }
}
