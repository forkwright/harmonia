// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Core.ImportLists.TMDb;

public class TMDbNowPlayingMovies : ImportListBase<TMDbSettings>
{
    private readonly TmdbInfoProxy _tmdbProxy;

    public TMDbNowPlayingMovies(TmdbInfoProxy tmdbProxy, ILogger<TMDbNowPlayingMovies> logger) : base(logger)
    {
        _tmdbProxy = tmdbProxy;
    }

    public override string Name => "TMDb Now Playing Movies";
    public override ImportListType ListType => ImportListType.TMDb;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(6);
    public override bool Enabled => true;
    public override bool EnableAuto => false;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            Logger.LogInformation("Fetching now playing movies from TMDb");
            var movies = await _tmdbProxy.GetNowPlayingAsync(cancellationToken);
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
        catch (Exception ex)
        {
            Logger.LogError(ex, "Error fetching TMDb now playing movies");
            return new ImportListFetchResult { AnyFailure = true };
        }
    }
}
