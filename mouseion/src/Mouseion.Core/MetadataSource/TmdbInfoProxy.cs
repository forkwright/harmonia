// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http;
using Mouseion.Core.Movies;

namespace Mouseion.Core.MetadataSource;

public class TmdbInfoProxy : IProvideMovieInfo
{
    private const string BaseUrl = "https://api.themoviedb.org/3";
    private readonly string _apiKey;

    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<TmdbInfoProxy> _logger;

    public TmdbInfoProxy(
        IHttpClient httpClient,
        IMemoryCache cache,
        IConfiguration configuration,
        ILogger<TmdbInfoProxy> logger)
    {
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;
        _apiKey = configuration["TMDb:ApiKey"] ?? Environment.GetEnvironmentVariable("TMDB_API_KEY") ?? string.Empty;

        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            _logger.LogWarning("TMDb API key not configured. Set TMDb:ApiKey in config or TMDB_API_KEY env var");
        }
    }

    public async Task<Movie?> GetByTmdbIdAsync(int tmdbId, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            _logger.LogWarning("TMDb API key not configured");
            return null;
        }

        var cacheKey = $"movie_tmdb_{tmdbId}";

        if (_cache.TryGetValue(cacheKey, out Movie? cached))
        {
            _logger.LogDebug("Cache hit for TMDb ID: {TmdbId}", tmdbId);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching movie by TMDb ID: {TmdbId}", tmdbId);

            var url = $"{BaseUrl}/movie/{tmdbId}?api_key={_apiKey}&append_to_response=credits,release_dates";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("TMDb returned {StatusCode} for TMDb ID {TmdbId}", response.StatusCode, tmdbId);
                return null;
            }

            var result = ParseMovie(response.Content);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching movie by TMDb ID: {TmdbId}", tmdbId);
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for movie TMDb ID: {TmdbId}", tmdbId);
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: movie by TMDb ID {TmdbId}", tmdbId);
            return null;
        }
    }

    public async Task<Movie?> GetByImdbIdAsync(string imdbId, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            _logger.LogWarning("TMDb API key not configured");
            return null;
        }

        var cacheKey = $"movie_imdb_{imdbId}";

        if (_cache.TryGetValue(cacheKey, out Movie? cached))
        {
            _logger.LogDebug("Cache hit for IMDB ID: {ImdbId}", imdbId);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching movie by IMDB ID: {ImdbId}", imdbId);

            var url = $"{BaseUrl}/find/{imdbId}?api_key={_apiKey}&external_source=imdb_id";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("TMDb returned {StatusCode} for IMDB ID {ImdbId}", response.StatusCode, imdbId);
                return null;
            }

            var result = ParseFindResult(response.Content);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching movie by IMDB ID: {ImdbId}", imdbId);
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for movie IMDB ID: {ImdbId}", imdbId);
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: movie by IMDB ID {ImdbId}", imdbId);
            return null;
        }
    }

    public async Task<List<Movie>> SearchByTitleAsync(string title, int? year = null, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            _logger.LogWarning("TMDb API key not configured");
            return new List<Movie>();
        }

        var cacheKey = $"movie_search_{title.ToLowerInvariant()}_{year}";

        if (_cache.TryGetValue(cacheKey, out List<Movie>? cached))
        {
            _logger.LogDebug("Cache hit for movie search: {Title}", title.SanitizeForLog());
            return cached ?? new List<Movie>();
        }

        try
        {
            _logger.LogDebug("Searching movies by title: {Title}", title.SanitizeForLog());

            var url = $"{BaseUrl}/search/movie?api_key={_apiKey}&query={Uri.EscapeDataString(title)}";
            if (year.HasValue)
            {
                url += $"&year={year.Value}";
            }

            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("TMDb search returned {StatusCode}", response.StatusCode);
                return new List<Movie>();
            }

            var result = ParseSearchResults(response.Content);
            _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching movies by title: {Title}", title.SanitizeForLog());
            return new List<Movie>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for movie title: {Title}", title.SanitizeForLog());
            return new List<Movie>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: movie title search {Title}", title.SanitizeForLog());
            return new List<Movie>();
        }
    }

    public async Task<List<Movie>> GetTrendingAsync(CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            return new List<Movie>();
        }

        try
        {
            var url = $"{BaseUrl}/trending/movie/week?api_key={_apiKey}";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                return new List<Movie>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching trending movies");
            return new List<Movie>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for trending movies");
            return new List<Movie>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: trending movies");
            return new List<Movie>();
        }
    }

    public async Task<List<Movie>> GetPopularAsync(CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            return new List<Movie>();
        }

        try
        {
            var url = $"{BaseUrl}/movie/popular?api_key={_apiKey}";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                return new List<Movie>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching popular movies");
            return new List<Movie>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for popular movies");
            return new List<Movie>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: popular movies");
            return new List<Movie>();
        }
    }

    public async Task<List<Movie>> GetUpcomingAsync(CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            return new List<Movie>();
        }

        try
        {
            var url = $"{BaseUrl}/movie/upcoming?api_key={_apiKey}";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                return new List<Movie>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching upcoming movies");
            return new List<Movie>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for upcoming movies");
            return new List<Movie>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: upcoming movies");
            return new List<Movie>();
        }
    }

    public async Task<List<Movie>> GetNowPlayingAsync(CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey))
        {
            return new List<Movie>();
        }

        try
        {
            var url = $"{BaseUrl}/movie/now_playing?api_key={_apiKey}";
            var request = new HttpRequestBuilder(url).Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                return new List<Movie>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching now playing movies");
            return new List<Movie>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for now playing movies");
            return new List<Movie>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: now playing movies");
            return new List<Movie>();
        }
    }

    private Movie? ParseMovie(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var movie = new Movie
            {
                Title = root.GetProperty("title").GetString() ?? "Unknown",
                Year = ExtractYear(root),
                Overview = JsonHelpers.GetStringProperty(root, "overview"),
                Runtime = JsonHelpers.GetIntProperty(root, "runtime"),
                TmdbId = JsonHelpers.GetIntProperty(root, "id")?.ToString(),
                ImdbId = JsonHelpers.GetStringProperty(root, "imdb_id"),
                Genres = GetGenres(root),
                Certification = ExtractCertification(root),
                Studio = JsonHelpers.GetStringProperty(root, "production_companies"),
                Website = JsonHelpers.GetStringProperty(root, "homepage"),
                Popularity = JsonHelpers.GetFloatProperty(root, "popularity"),
                Images = GetImages(root)
            };

            if (root.TryGetProperty("release_date", out var releaseDate) &&
                DateTime.TryParse(releaseDate.GetString(), out var date))
            {
                movie.PhysicalRelease = date;
            }

            return movie;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing TMDb movie JSON");
            return null;
        }
    }

    private Movie? ParseFindResult(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("movie_results", out var results) &&
                results.GetArrayLength() > 0)
            {
                var first = results[0];
                return ParseMovieFromSearchResult(first);
            }

            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing TMDb find result");
            return null;
        }
    }

    private List<Movie> ParseSearchResults(string json)
    {
        var movies = new List<Movie>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("results", out var results))
            {
                foreach (var item in results.EnumerateArray())
                {
                    var movie = ParseMovieFromSearchResult(item);
                    if (movie != null)
                    {
                        movies.Add(movie);
                    }
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing TMDb search results");
        }

        return movies;
    }

    private Movie? ParseMovieFromSearchResult(JsonElement item)
    {
        try
        {
            var tmdbId = JsonHelpers.GetIntProperty(item, "id");
            if (!tmdbId.HasValue)
            {
                return null;
            }

            var movie = new Movie
            {
                Title = item.GetProperty("title").GetString() ?? "Unknown",
                Year = ExtractYear(item),
                Overview = JsonHelpers.GetStringProperty(item, "overview"),
                TmdbId = tmdbId.Value.ToString(),
                Genres = GetGenres(item),
                Popularity = JsonHelpers.GetFloatProperty(item, "popularity"),
                Images = GetImages(item)
            };

            if (item.TryGetProperty("release_date", out var releaseDate) &&
                DateTime.TryParse(releaseDate.GetString(), out var date))
            {
                movie.PhysicalRelease = date;
            }

            return movie;
        }
        catch (JsonException ex)
        {
            _logger.LogWarning(ex, "Error parsing movie from search result");
            return null;
        }
    }


    private static int ExtractYear(JsonElement element)
    {
        var releaseDate = JsonHelpers.GetStringProperty(element, "release_date");
        if (string.IsNullOrWhiteSpace(releaseDate))
        {
            return 0;
        }

        if (DateTime.TryParse(releaseDate, out var date))
        {
            return date.Year;
        }

        return 0;
    }

    private static List<string> GetGenres(JsonElement element)
    {
        var genres = new List<string>();

        if (element.TryGetProperty("genres", out var genresArray) && genresArray.ValueKind == JsonValueKind.Array)
        {
            foreach (var genre in genresArray.EnumerateArray())
            {
                if (genre.TryGetProperty("name", out var name) && name.ValueKind == JsonValueKind.String)
                {
                    var value = name.GetString();
                    if (!string.IsNullOrWhiteSpace(value))
                    {
                        genres.Add(value);
                    }
                }
            }
        }
        else if (element.TryGetProperty("genre_ids", out var genreIds) && genreIds.ValueKind == JsonValueKind.Array)
        {
            foreach (var genreId in genreIds.EnumerateArray())
            {
                if (genreId.ValueKind == JsonValueKind.Number)
                {
                    genres.Add(genreId.GetInt32().ToString());
                }
            }
        }

        return genres;
    }

    private static List<string> GetImages(JsonElement element)
    {
        var images = new List<string>();

        var posterPath = JsonHelpers.GetStringProperty(element, "poster_path");
        if (!string.IsNullOrWhiteSpace(posterPath))
        {
            images.Add($"https://image.tmdb.org/t/p/original{posterPath}");
        }

        var backdropPath = JsonHelpers.GetStringProperty(element, "backdrop_path");
        if (!string.IsNullOrWhiteSpace(backdropPath))
        {
            images.Add($"https://image.tmdb.org/t/p/original{backdropPath}");
        }

        return images;
    }

    private static string? ExtractCertification(JsonElement element)
    {
        if (element.TryGetProperty("release_dates", out var releaseDates) &&
            releaseDates.TryGetProperty("results", out var results))
        {
            foreach (var country in results.EnumerateArray())
            {
                if (country.TryGetProperty("iso_3166_1", out var iso) &&
                    iso.GetString() == "US" &&
                    country.TryGetProperty("release_dates", out var dates))
                {
                    foreach (var date in dates.EnumerateArray())
                    {
                        var cert = JsonHelpers.GetStringProperty(date, "certification");
                        if (!string.IsNullOrWhiteSpace(cert))
                        {
                            return cert;
                        }
                    }
                }
            }
        }

        return null;
    }
}
