// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.AniList;

/// <summary>
/// AniList import list. GraphQL API, supports both authenticated (OAuth token)
/// and public (username-only) access. Imports anime and manga lists.
/// </summary>
public class AniListImportList : ImportListBase<AniListSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;

    // GraphQL query for media list collection
    private const string MediaListQuery = @"
query ($userName: String, $type: MediaType, $chunk: Int, $perChunk: Int) {
  MediaListCollection(userName: $userName, type: $type, chunk: $chunk, perChunk: $perChunk) {
    lists {
      name
      status
      entries {
        id
        status
        score(format: POINT_10)
        progress
        progressVolumes
        startedAt { year month day }
        completedAt { year month day }
        updatedAt
        media {
          id
          idMal
          title { romaji english native }
          type
          format
          startDate { year month day }
          episodes
          chapters
          volumes
        }
      }
    }
    hasNextChunk
  }
}";

    public AniListImportList(ILogger<AniListImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "AniList";
    public override ImportListType ListType => ImportListType.AniList;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(1);
    public override bool Enabled => Settings.HasValidCredentials;
    public override bool EnableAuto => Settings.HasValidCredentials;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        var result = new ImportListFetchResult();
        var items = new List<ImportListItem>();

        try
        {
            if (!Settings.HasValidCredentials)
            {
                Logger.LogWarning("AniList credentials not configured. Set username or OAuth token.");
                result.AnyFailure = true;
                return result;
            }

            var client = CreateClient();

            if (Settings.ImportAnimeList)
            {
                var animeItems = await FetchMediaListAsync(client, "ANIME", cancellationToken);
                items.AddRange(animeItems);
            }

            if (Settings.ImportMangaList)
            {
                var mangaItems = await FetchMediaListAsync(client, "MANGA", cancellationToken);
                items.AddRange(mangaItems);
            }

            // Deduplicate by AniList ID
            var uniqueItems = items
                .GroupBy(i => (i.AniListId, i.MediaType))
                .Select(g => g.First())
                .ToList();

            result.Items = CleanupListItems(uniqueItems);
            result.SyncedLists = (Settings.ImportAnimeList ? 1 : 0) + (Settings.ImportMangaList ? 1 : 0);
        }
        catch (HttpRequestException ex)
        {
            Logger.LogError(ex, "Failed to fetch AniList data");
            result.AnyFailure = true;
        }

        return result;
    }

    private async Task<List<ImportListItem>> FetchMediaListAsync(
        HttpClient client,
        string mediaType,
        CancellationToken cancellationToken)
    {
        var items = new List<ImportListItem>();
        var chunk = 1;
        var hasMore = true;

        while (hasMore)
        {
            var variables = new Dictionary<string, object>
            {
                ["userName"] = Settings.Username,
                ["type"] = mediaType,
                ["chunk"] = chunk,
                ["perChunk"] = 500
            };

            var requestBody = new
            {
                query = MediaListQuery,
                variables
            };

            var json = JsonSerializer.Serialize(requestBody);
            using var content = new StringContent(json, Encoding.UTF8, "application/json");
            using var response = await client.PostAsync(Settings.BaseUrl, content, cancellationToken);
            response.EnsureSuccessStatusCode();

            var responseText = await response.Content.ReadAsStringAsync(cancellationToken);
            var graphqlResponse = JsonSerializer.Deserialize<AniListGraphQLResponse<AniListMediaListCollectionData>>(responseText);

            if (graphqlResponse?.Errors?.Count > 0)
            {
                var errorMsg = string.Join("; ", graphqlResponse.Errors.Select(e => e.Message));
                Logger.LogError("AniList GraphQL errors: {Errors}", errorMsg);
                break;
            }

            var collection = graphqlResponse?.Data?.MediaListCollection;
            if (collection?.Lists == null) break;

            foreach (var list in collection.Lists)
            {
                foreach (var entry in list.Entries)
                {
                    if (ShouldFilter(entry.Status)) continue;
                    if (entry.Media == null) continue;

                    var isAnime = mediaType == "ANIME";
                    var title = entry.Media.Title?.English
                        ?? entry.Media.Title?.Romaji
                        ?? entry.Media.Title?.Native
                        ?? string.Empty;

                    items.Add(new ImportListItem
                    {
                        MediaType = isAnime ? MediaType.Movie : MediaType.Manga,
                        Title = title,
                        AniListId = entry.Media.Id,
                        MalId = entry.Media.IdMal,
                        Year = entry.Media.StartDate?.Year ?? 0,
                        UserRating = entry.Score > 0 ? (int)Math.Round(entry.Score) : null,
                        WatchedAt = entry.CompletedAt?.ToDateTime(),
                        ImportSource = "AniList"
                    });
                }
            }

            hasMore = collection.HasNextChunk;
            chunk++;
        }

        Logger.LogInformation("Fetched {Count} {Type} from AniList", items.Count, mediaType.ToLowerInvariant());
        return items;
    }

    private bool ShouldFilter(string? status)
    {
        if (string.IsNullOrEmpty(status) || Settings.StatusFilter.Count == 0)
        {
            return false;
        }

        return !Settings.StatusFilter.Contains(status, StringComparer.OrdinalIgnoreCase);
    }

    private HttpClient CreateClient()
    {
        var client = _httpClientFactory.CreateClient("AniList");
        client.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));

        if (!string.IsNullOrEmpty(Settings.AccessToken))
        {
            client.DefaultRequestHeaders.Authorization =
                new AuthenticationHeaderValue("Bearer", Settings.AccessToken);
        }

        return client;
    }
}
