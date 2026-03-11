// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.Datastore;
using Mouseion.Core.Movies;
using Mouseion.Core.Music;
using Mouseion.Core.Podcasts;
using Mouseion.Core.TV;

namespace Mouseion.Core.Search;

public interface IUnifiedSearchService
{
    Task<UnifiedSearchResult> SearchAsync(string query, int limit, string? mediaType = null, CancellationToken ct = default);
}

public class UnifiedSearchService : IUnifiedSearchService
{
    private readonly ITrackSearchService _trackSearchService;
    private readonly IMovieRepository _movieRepository;
    private readonly ISeriesRepository _seriesRepository;
    private readonly IBookRepository _bookRepository;
    private readonly IAudiobookRepository _audiobookRepository;
    private readonly IPodcastShowRepository _podcastShowRepository;

    public UnifiedSearchService(
        ITrackSearchService trackSearchService,
        IMovieRepository movieRepository,
        ISeriesRepository seriesRepository,
        IBookRepository bookRepository,
        IAudiobookRepository audiobookRepository,
        IPodcastShowRepository podcastShowRepository)
    {
        _trackSearchService = trackSearchService;
        _movieRepository = movieRepository;
        _seriesRepository = seriesRepository;
        _bookRepository = bookRepository;
        _audiobookRepository = audiobookRepository;
        _podcastShowRepository = podcastShowRepository;
    }

    public async Task<UnifiedSearchResult> SearchAsync(string query, int limit, string? mediaType = null, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(query))
        {
            return UnifiedSearchResult.Empty;
        }

        var normalizedQuery = query.Trim();
        var perTypeLimit = Math.Max(limit / 3, 10); // Distribute limit across types, floor of 10

        var result = new UnifiedSearchResult();
        var tasks = new List<Task>();

        // Filter by media type if specified, otherwise search everything
        var searchAll = string.IsNullOrEmpty(mediaType);

        if (searchAll || mediaType!.Equals("music", StringComparison.OrdinalIgnoreCase) ||
            mediaType!.Equals("track", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchTracksAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        if (searchAll || mediaType!.Equals("movie", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchMoviesAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        if (searchAll || mediaType!.Equals("tv", StringComparison.OrdinalIgnoreCase) ||
            mediaType!.Equals("series", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchSeriesAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        if (searchAll || mediaType!.Equals("book", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchBooksAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        if (searchAll || mediaType!.Equals("audiobook", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchAudiobooksAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        if (searchAll || mediaType!.Equals("podcast", StringComparison.OrdinalIgnoreCase))
        {
            tasks.Add(SearchPodcastsAsync(normalizedQuery, perTypeLimit, result, ct));
        }

        await Task.WhenAll(tasks).ConfigureAwait(false);

        return result;
    }

    private async Task SearchTracksAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var tracks = await _trackSearchService.SearchAsync(query, limit, ct).ConfigureAwait(false);
            lock (result)
            {
                result.Tracks.AddRange(tracks.Select(t => new SearchHit
                {
                    Id = t.Track.Id,
                    MediaType = "track",
                    Title = t.Track.Title,
                    Subtitle = FormatTrackSubtitle(t.ArtistName, t.AlbumTitle),
                    Year = 0,
                    Score = t.RelevanceScore,
                    Extra = new Dictionary<string, object?>
                    {
                        ["artistName"] = t.ArtistName,
                        ["albumTitle"] = t.AlbumTitle,
                        ["trackNumber"] = t.Track.TrackNumber,
                        ["discNumber"] = t.Track.DiscNumber,
                        ["durationSeconds"] = t.Track.DurationSeconds,
                        ["genre"] = t.Genre,
                        ["lossless"] = t.Lossless,
                        ["bitDepth"] = t.BitDepth
                    }
                }));
            }
        }
        catch (Exception)
        {
            // Don't let one media type failure kill the whole search
        }
    }

    private async Task SearchMoviesAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var all = await _movieRepository.AllAsync(ct).ConfigureAwait(false);
            var scored = ScoreAndRank(all, query, m => m.Title, m => m.Genres, limit);
            lock (result)
            {
                result.Movies.AddRange(scored.Select(s => new SearchHit
                {
                    Id = s.Item.Id,
                    MediaType = "movie",
                    Title = s.Item.Title,
                    Subtitle = s.Item.Studio,
                    Year = s.Item.Year,
                    Score = s.Score,
                    Extra = new Dictionary<string, object?>
                    {
                        ["tmdbId"] = s.Item.TmdbId,
                        ["imdbId"] = s.Item.ImdbId,
                        ["runtime"] = s.Item.Runtime,
                        ["genres"] = s.Item.Genres
                    }
                }));
            }
        }
        catch (Exception)
        {
        }
    }

    private async Task SearchSeriesAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var all = await _seriesRepository.AllAsync(ct).ConfigureAwait(false);
            var scored = ScoreAndRank(all, query, s => s.Title, s => s.Genres, limit);
            lock (result)
            {
                result.Series.AddRange(scored.Select(s => new SearchHit
                {
                    Id = s.Item.Id,
                    MediaType = "series",
                    Title = s.Item.Title,
                    Subtitle = s.Item.Network,
                    Year = s.Item.Year,
                    Score = s.Score,
                    Extra = new Dictionary<string, object?>
                    {
                        ["tvdbId"] = s.Item.TvdbId,
                        ["status"] = s.Item.Status,
                        ["network"] = s.Item.Network,
                        ["genres"] = s.Item.Genres
                    }
                }));
            }
        }
        catch (Exception)
        {
        }
    }

    private async Task SearchBooksAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var all = await _bookRepository.AllAsync(ct).ConfigureAwait(false);
            var scored = ScoreAndRank(all, query, b => b.Title, _ => new List<string>(), limit);
            lock (result)
            {
                result.Books.AddRange(scored.Select(s => new SearchHit
                {
                    Id = s.Item.Id,
                    MediaType = "book",
                    Title = s.Item.Title,
                    Subtitle = null,
                    Year = s.Item.Year,
                    Score = s.Score
                }));
            }
        }
        catch (Exception)
        {
        }
    }

    private async Task SearchAudiobooksAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var all = await _audiobookRepository.AllAsync(ct).ConfigureAwait(false);
            var scored = ScoreAndRank(all, query, a => a.Title, _ => new List<string>(), limit);
            lock (result)
            {
                result.Audiobooks.AddRange(scored.Select(s => new SearchHit
                {
                    Id = s.Item.Id,
                    MediaType = "audiobook",
                    Title = s.Item.Title,
                    Subtitle = null,
                    Year = s.Item.Year,
                    Score = s.Score
                }));
            }
        }
        catch (Exception)
        {
        }
    }

    private async Task SearchPodcastsAsync(string query, int limit, UnifiedSearchResult result, CancellationToken ct)
    {
        try
        {
            var all = await _podcastShowRepository.AllAsync(ct).ConfigureAwait(false);
            var scored = ScoreAndRank(all, query,
                p => p.Title,
                p => ParseCategories(p.Categories),
                limit);
            lock (result)
            {
                result.Podcasts.AddRange(scored.Select(s => new SearchHit
                {
                    Id = s.Item.Id,
                    MediaType = "podcast",
                    Title = s.Item.Title,
                    Subtitle = s.Item.Author,
                    Year = 0,
                    Score = s.Score,
                    Extra = new Dictionary<string, object?>
                    {
                        ["author"] = s.Item.Author,
                        ["episodeCount"] = s.Item.EpisodeCount
                    }
                }));
            }
        }
        catch (Exception)
        {
        }
    }

    // Generic scoring for any entity with a title and optional genres
    private static List<ScoredItem<T>> ScoreAndRank<T>(
        IEnumerable<T> items,
        string query,
        Func<T, string> getTitle,
        Func<T, IList<string>> getGenres,
        int limit)
    {
        var normalizedQuery = query.ToLowerInvariant();
        var queryWords = normalizedQuery.Split(' ', StringSplitOptions.RemoveEmptyEntries);

        var scored = new List<ScoredItem<T>>();

        foreach (var item in items)
        {
            var title = getTitle(item);
            var genres = getGenres(item);

            var score = ScoreTitle(title, normalizedQuery, queryWords);

            // Genre boost
            if (genres.Any(g => g.Contains(normalizedQuery, StringComparison.OrdinalIgnoreCase)))
            {
                score = Math.Max(score, 35.0);
            }

            if (score > 0)
            {
                scored.Add(new ScoredItem<T> { Item = item, Score = score });
            }
        }

        return scored
            .OrderByDescending(s => s.Score)
            .Take(limit)
            .ToList();
    }

    private static double ScoreTitle(string title, string normalizedQuery, string[] queryWords)
    {
        var titleLower = title?.ToLowerInvariant() ?? "";

        // Exact match
        if (titleLower == normalizedQuery) return 100.0;

        // Starts with
        if (titleLower.StartsWith(normalizedQuery)) return 70.0;

        // Contains
        if (titleLower.Contains(normalizedQuery)) return 50.0;

        // Multi-word: all words present
        if (queryWords.Length > 1 && queryWords.All(w => titleLower.Contains(w)))
        {
            return 40.0;
        }

        // Partial: any word present
        if (queryWords.Any(w => titleLower.Contains(w)))
        {
            return 20.0;
        }

        return 0.0;
    }

    private static List<string> ParseCategories(string? categories)
    {
        if (string.IsNullOrWhiteSpace(categories)) return new List<string>();
        try
        {
            return categories.Trim('[', ']')
                .Split(',')
                .Select(c => c.Trim(' ', '"'))
                .Where(c => !string.IsNullOrWhiteSpace(c))
                .ToList();
        }
        catch
        {
            return new List<string>();
        }
    }

    private static string? FormatTrackSubtitle(string? artist, string? album)
    {
        if (artist != null && album != null) return $"{artist} — {album}";
        return artist ?? album;
    }

    private class ScoredItem<T>
    {
        public T Item { get; set; } = default!;
        public double Score { get; set; }
    }
}

public class UnifiedSearchResult
{
    public List<SearchHit> Tracks { get; set; } = new();
    public List<SearchHit> Movies { get; set; } = new();
    public List<SearchHit> Series { get; set; } = new();
    public List<SearchHit> Books { get; set; } = new();
    public List<SearchHit> Audiobooks { get; set; } = new();
    public List<SearchHit> Podcasts { get; set; } = new();

    public int TotalCount => Tracks.Count + Movies.Count + Series.Count +
                             Books.Count + Audiobooks.Count + Podcasts.Count;

    public static UnifiedSearchResult Empty => new();
}

public class SearchHit
{
    public int Id { get; set; }
    public string MediaType { get; set; } = null!;
    public string Title { get; set; } = null!;
    public string? Subtitle { get; set; }
    public int Year { get; set; }
    public double Score { get; set; }
    public Dictionary<string, object?>? Extra { get; set; }
}
