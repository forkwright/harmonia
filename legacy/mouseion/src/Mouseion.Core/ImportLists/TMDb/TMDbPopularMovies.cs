// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Core.ImportLists.TMDb;

public class TMDbPopularMovies : ImportListBase<TMDbSettings>
{
    private readonly TmdbInfoProxy _tmdbProxy;

    public TMDbPopularMovies(TmdbInfoProxy tmdbProxy, ILogger<TMDbPopularMovies> logger) : base(logger)
    {
        _tmdbProxy = tmdbProxy;
    }

    public override string Name => "TMDb Popular Movies";
    public override ImportListType ListType => ImportListType.TMDb;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(12);
    public override bool Enabled => true;
    public override bool EnableAuto => false;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            Logger.LogInformation("Fetching popular movies from TMDb");
            var movies = await _tmdbProxy.GetPopularAsync(cancellationToken);
            var items = movies.Select(movie => new ImportListItem
            {
                MediaType = MediaType.Movie,
                Title = movie.Title,
                Year = movie.Year,
                TmdbId = int.TryParse(movie.TmdbId, out var id) ? id : 0,
                ImdbId = movie.ImdbId
            }).ToList();
            return new ImportListFetchResult { Items = CleanupListItems(items), SyncedLists = 1 };
        }
        catch (HttpRequestException ex)
        {
            Logger.LogError(ex, "Network error fetching TMDb popular movies");
            return new ImportListFetchResult { AnyFailure = true };
        }
        catch (JsonException ex)
        {
            Logger.LogError(ex, "Failed to parse TMDb popular movies response");
            return new ImportListFetchResult { AnyFailure = true };
        }
        catch (TaskCanceledException ex)
        {
            Logger.LogWarning(ex, "Request timed out or was cancelled fetching TMDb popular movies");
            return new ImportListFetchResult { AnyFailure = true };
        }
    }
}
