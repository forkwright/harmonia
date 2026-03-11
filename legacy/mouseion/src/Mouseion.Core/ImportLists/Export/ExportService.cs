// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;
using Mouseion.Core.Books;

namespace Mouseion.Core.ImportLists.Export;

/// <summary>
/// Export library data in portable formats.
/// Reduces lock-in anxiety — users who can export are more willing to commit.
/// </summary>
public interface IExportService
{
    /// <summary>Export library as JSON (complete, machine-readable).</summary>
    Task<ExportResult> ExportJsonAsync(ExportOptions options, CancellationToken ct = default);

    /// <summary>Export library as CSV (spreadsheet-friendly).</summary>
    Task<ExportResult> ExportCsvAsync(ExportOptions options, CancellationToken ct = default);

    /// <summary>Export in a format compatible with another service (Trakt, Goodreads, etc.).</summary>
    Task<ExportResult> ExportForServiceAsync(ExportTarget target, ExportOptions options, CancellationToken ct = default);
}

public class ExportService : IExportService
{
    private readonly IMediaItemRepository _mediaItemRepository;
    private readonly IMovieRepository _movieRepository;
    private readonly IBookRepository _bookRepository;
    private readonly ILogger<ExportService> _logger;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public ExportService(
        IMediaItemRepository mediaItemRepository,
        IMovieRepository movieRepository,
        IBookRepository bookRepository,
        ILogger<ExportService> logger)
    {
        _mediaItemRepository = mediaItemRepository;
        _movieRepository = movieRepository;
        _bookRepository = bookRepository;
        _logger = logger;
    }

    public async Task<ExportResult> ExportJsonAsync(ExportOptions options, CancellationToken ct = default)
    {
        var items = await GatherExportItemsAsync(options, ct);

        var export = new MouseionExport
        {
            Version = "1.0",
            ExportedAt = DateTime.UtcNow,
            MediaTypes = options.MediaTypes?.ToList(),
            Items = items
        };

        var json = JsonSerializer.Serialize(export, JsonOptions);

        _logger.LogInformation("Exported {Count} items as JSON ({MediaTypes})",
            items.Count, options.MediaTypes != null ? string.Join(", ", options.MediaTypes) : "all");

        return new ExportResult
        {
            FileName = $"mouseion-export-{DateTime.UtcNow:yyyy-MM-dd}.json",
            ContentType = "application/json",
            Data = Encoding.UTF8.GetBytes(json),
            ItemCount = items.Count
        };
    }

    public async Task<ExportResult> ExportCsvAsync(ExportOptions options, CancellationToken ct = default)
    {
        var items = await GatherExportItemsAsync(options, ct);
        var sb = new StringBuilder();

        // Header
        sb.AppendLine("MediaType,Title,Year,Author/Artist,ExternalId,ExternalIdType,Rating,Added,Status");

        foreach (var item in items)
        {
            var escapedTitle = CsvEscape(item.Title);
            var escapedCreator = CsvEscape(item.Creator ?? "");
            sb.AppendLine($"{item.MediaType},{escapedTitle},{item.Year},{escapedCreator},{item.PrimaryExternalId},{item.ExternalIdType},{item.Rating},{item.Added:yyyy-MM-dd},{item.Status}");
        }

        _logger.LogInformation("Exported {Count} items as CSV", items.Count);

        return new ExportResult
        {
            FileName = $"mouseion-export-{DateTime.UtcNow:yyyy-MM-dd}.csv",
            ContentType = "text/csv",
            Data = Encoding.UTF8.GetBytes(sb.ToString()),
            ItemCount = items.Count
        };
    }

    public async Task<ExportResult> ExportForServiceAsync(ExportTarget target, ExportOptions options, CancellationToken ct = default)
    {
        var items = await GatherExportItemsAsync(options, ct);

        return target switch
        {
            ExportTarget.TraktImport => ExportTraktFormat(items),
            ExportTarget.GoodreadsCsv => ExportGoodreadsFormat(items),
            ExportTarget.LetterboxdCsv => ExportLetterboxdFormat(items),
            ExportTarget.GenericJson => await ExportJsonAsync(options, ct),
            _ => throw new ArgumentException($"Unknown export target: {target}")
        };
    }

    private async Task<List<ExportItem>> GatherExportItemsAsync(ExportOptions options, CancellationToken ct)
    {
        var items = new List<ExportItem>();
        var mediaTypes = options.MediaTypes ?? Enum.GetValues<MediaType>().Where(t => t != MediaType.Unknown);
        int page = 1;
        const int pageSize = 500;

        foreach (var mediaType in mediaTypes)
        {
            page = 1;
            bool hasMore = true;

            while (hasMore)
            {
                var summaries = await _mediaItemRepository.GetPageAsync(page, pageSize, mediaType, ct);
                if (summaries.Count == 0)
                {
                    hasMore = false;
                    continue;
                }

                foreach (var summary in summaries)
                {
                    var exportItem = new ExportItem
                    {
                        MediaType = summary.MediaType,
                        Title = summary.Title,
                        Year = summary.Year,
                        Added = summary.Added,
                        Status = summary.Monitored ? "monitored" : "unmonitored"
                    };

                    // Enrich with media-type-specific data
                    await EnrichExportItemAsync(exportItem, summary, ct);
                    items.Add(exportItem);
                }

                page++;
                if (summaries.Count < pageSize) hasMore = false;
            }
        }

        // Apply date filter
        if (options.AddedAfter.HasValue)
        {
            items = items.Where(i => i.Added >= options.AddedAfter.Value).ToList();
        }

        return items;
    }

    private async Task EnrichExportItemAsync(ExportItem exportItem, MediaItemSummary summary, CancellationToken ct)
    {
        switch (summary.MediaType)
        {
            case MediaType.Movie:
                var movie = await _movieRepository.FindByTmdbIdAsync(summary.Id.ToString(), ct);
                if (movie != null)
                {
                    exportItem.PrimaryExternalId = movie.TmdbId;
                    exportItem.ExternalIdType = "tmdb";
                    exportItem.SecondaryExternalId = movie.ImdbId;
                }
                break;

            case MediaType.Book:
            case MediaType.Audiobook:
                var book = await _bookRepository.FindByTitleAsync(summary.Title, summary.Year, ct);
                if (book != null)
                {
                    exportItem.PrimaryExternalId = book.Metadata?.Isbn13 ?? book.Metadata?.Isbn;
                    exportItem.ExternalIdType = "isbn";
                    exportItem.SecondaryExternalId = book.Metadata?.GoodreadsId;
                }
                break;
        }
    }

    private ExportResult ExportTraktFormat(List<ExportItem> items)
    {
        // Trakt import format: JSON array of {title, year, ids: {tmdb, imdb}}
        var traktItems = items
            .Where(i => i.MediaType is MediaType.Movie or MediaType.TV)
            .Select(i => new
            {
                title = i.Title,
                year = i.Year,
                ids = new
                {
                    tmdb = i.ExternalIdType == "tmdb" ? int.TryParse(i.PrimaryExternalId, out var id) ? id : (int?)null : null,
                    imdb = i.SecondaryExternalId ?? (i.ExternalIdType == "imdb" ? i.PrimaryExternalId : null)
                }
            }).ToList();

        var json = JsonSerializer.Serialize(new { movies = traktItems }, JsonOptions);

        return new ExportResult
        {
            FileName = $"mouseion-trakt-export-{DateTime.UtcNow:yyyy-MM-dd}.json",
            ContentType = "application/json",
            Data = Encoding.UTF8.GetBytes(json),
            ItemCount = traktItems.Count
        };
    }

    private ExportResult ExportGoodreadsFormat(List<ExportItem> items)
    {
        // Goodreads CSV: Title, Author, ISBN, My Rating, Date Added, Bookshelves
        var sb = new StringBuilder();
        sb.AppendLine("Title,Author,ISBN,My Rating,Date Added,Bookshelves");

        var books = items.Where(i => i.MediaType is MediaType.Book or MediaType.Audiobook);
        foreach (var book in books)
        {
            sb.AppendLine($"{CsvEscape(book.Title)},{CsvEscape(book.Creator ?? "")},{book.PrimaryExternalId},{book.Rating},{book.Added:yyyy/MM/dd},{book.Status}");
        }

        return new ExportResult
        {
            FileName = $"mouseion-goodreads-export-{DateTime.UtcNow:yyyy-MM-dd}.csv",
            ContentType = "text/csv",
            Data = Encoding.UTF8.GetBytes(sb.ToString()),
            ItemCount = items.Count(i => i.MediaType is MediaType.Book or MediaType.Audiobook)
        };
    }

    private ExportResult ExportLetterboxdFormat(List<ExportItem> items)
    {
        // Letterboxd CSV: Title, Year, Rating10, WatchedDate, imdbID, tmdbID
        var sb = new StringBuilder();
        sb.AppendLine("Title,Year,Rating10,WatchedDate,imdbID,tmdbID");

        var movies = items.Where(i => i.MediaType == MediaType.Movie);
        foreach (var movie in movies)
        {
            var tmdb = movie.ExternalIdType == "tmdb" ? movie.PrimaryExternalId : "";
            var imdb = movie.SecondaryExternalId ?? (movie.ExternalIdType == "imdb" ? movie.PrimaryExternalId : "");
            sb.AppendLine($"{CsvEscape(movie.Title)},{movie.Year},{movie.Rating},,{imdb},{tmdb}");
        }

        return new ExportResult
        {
            FileName = $"mouseion-letterboxd-export-{DateTime.UtcNow:yyyy-MM-dd}.csv",
            ContentType = "text/csv",
            Data = Encoding.UTF8.GetBytes(sb.ToString()),
            ItemCount = items.Count(i => i.MediaType == MediaType.Movie)
        };
    }

    private static string CsvEscape(string value)
    {
        if (value.Contains(',') || value.Contains('"') || value.Contains('\n'))
        {
            return $"\"{value.Replace("\"", "\"\"")}\"";
        }
        return value;
    }
}

public class MouseionExport
{
    public string Version { get; set; } = "1.0";
    public DateTime ExportedAt { get; set; }
    public List<MediaType>? MediaTypes { get; set; }
    public List<ExportItem> Items { get; set; } = new();
}

public class ExportItem
{
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public string? Creator { get; set; } // Author, artist, director
    public string? PrimaryExternalId { get; set; }
    public string? ExternalIdType { get; set; }
    public string? SecondaryExternalId { get; set; }
    public int? Rating { get; set; }
    public DateTime Added { get; set; }
    public string Status { get; set; } = string.Empty;
}

public class ExportOptions
{
    public IEnumerable<MediaType>? MediaTypes { get; set; }
    public DateTime? AddedAfter { get; set; }
}

public class ExportResult
{
    public string FileName { get; set; } = string.Empty;
    public string ContentType { get; set; } = string.Empty;
    public byte[] Data { get; set; } = Array.Empty<byte>();
    public int ItemCount { get; set; }
}

public enum ExportTarget
{
    GenericJson = 0,
    TraktImport = 1,
    GoodreadsCsv = 2,
    LetterboxdCsv = 3
}
