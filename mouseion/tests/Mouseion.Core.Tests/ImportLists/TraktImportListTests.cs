// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Moq;
using Moq.Protected;
using Mouseion.Core.ImportLists;
using Mouseion.Core.ImportLists.Trakt;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tests.ImportLists;

public class TraktImportListTests
{
    private readonly Mock<ILogger<TraktImportList>> _loggerMock;

    public TraktImportListTests()
    {
        _loggerMock = new Mock<ILogger<TraktImportList>>();
    }

    private TraktImportList CreateImportList(HttpClient httpClient, TraktSettings? settings = null)
    {
        var httpFactory = new Mock<IHttpClientFactory>();
        httpFactory.Setup(f => f.CreateClient(It.IsAny<string>())).Returns(httpClient);

        var importList = new TraktImportList(_loggerMock.Object, httpFactory.Object);

        settings ??= new TraktSettings
        {
            ClientId = "test-client-id",
            ClientSecret = "test-secret",
            AccessToken = "test-access-token",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30)
        };

        importList.Definition = new ImportListDefinition
        {
            Id = 1,
            Settings = JsonSerializer.Serialize(settings)
        };

        return importList;
    }

    [Fact]
    public void Name_IsTrakt()
    {
        var httpFactory = new Mock<IHttpClientFactory>();
        var importList = new TraktImportList(_loggerMock.Object, httpFactory.Object);
        Assert.Equal("Trakt", importList.Name);
    }

    [Fact]
    public void ListType_IsTrakt()
    {
        var httpFactory = new Mock<IHttpClientFactory>();
        var importList = new TraktImportList(_loggerMock.Object, httpFactory.Object);
        Assert.Equal(ImportListType.Trakt, importList.ListType);
    }

    [Fact]
    public async Task FetchAsync_NoToken_ReturnsFailure()
    {
        var handler = new Mock<HttpMessageHandler>();
        var client = new HttpClient(handler.Object);

        var importList = CreateImportList(client, new TraktSettings
        {
            ClientId = "test",
            AccessToken = "" // No token
        });

        var result = await importList.FetchAsync();
        Assert.True(result.AnyFailure);
        Assert.Empty(result.Items);
    }

    [Fact]
    public async Task FetchAsync_WithWatchlist_MapsMovies()
    {
        var watchlistItems = new List<TraktWatchlistItem>
        {
            new()
            {
                Rank = 1,
                ListedAt = DateTime.UtcNow,
                Type = "movie",
                Movie = new TraktMovie
                {
                    Title = "Inception",
                    Year = 2010,
                    Ids = new TraktIds { Tmdb = 27205, Imdb = "tt1375666" }
                }
            },
            new()
            {
                Rank = 2,
                ListedAt = DateTime.UtcNow,
                Type = "show",
                Show = new TraktShow
                {
                    Title = "Breaking Bad",
                    Year = 2008,
                    Ids = new TraktIds { Tvdb = 81189, Tmdb = 1396, Imdb = "tt0903747" }
                }
            }
        };

        var handler = CreateMockHandler(new Dictionary<string, string>
        {
            ["users/me/watchlist/movies,shows"] = JsonSerializer.Serialize(watchlistItems),
            ["users/me/collection/movies"] = "[]",
            ["users/me/collection/shows"] = "[]",
            ["users/me/history/movies,shows"] = "[]",
            ["users/me/ratings/movies,shows"] = "[]"
        });

        var client = new HttpClient(handler.Object) { BaseAddress = new Uri("https://api.trakt.tv") };

        var settings = new TraktSettings
        {
            ClientId = "test",
            AccessToken = "valid-token",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30),
            ImportWatchlist = true,
            ImportCollection = true,
            ImportWatchHistory = true,
            ImportRatings = true
        };

        var importList = CreateImportList(client, settings);
        var result = await importList.FetchAsync();

        Assert.False(result.AnyFailure);
        Assert.Equal(2, result.Items.Count);

        var movie = result.Items.First(i => i.MediaType == MediaType.Movie);
        Assert.Equal("Inception", movie.Title);
        Assert.Equal(27205, movie.TmdbId);
        Assert.Equal("tt1375666", movie.ImdbId);

        var show = result.Items.First(i => i.MediaType == MediaType.TV);
        Assert.Equal("Breaking Bad", show.Title);
        Assert.Equal(81189, show.TvdbId);
    }

    [Fact]
    public async Task FetchAsync_DuplicateItems_Deduplicates()
    {
        var watchlistItems = new List<TraktWatchlistItem>
        {
            new()
            {
                Rank = 1,
                Type = "movie",
                Movie = new TraktMovie
                {
                    Title = "Inception",
                    Year = 2010,
                    Ids = new TraktIds { Tmdb = 27205 }
                }
            }
        };

        var ratingItems = new List<TraktRatingItem>
        {
            new()
            {
                Rating = 9,
                Type = "movie",
                Movie = new TraktMovie
                {
                    Title = "Inception",
                    Year = 2010,
                    Ids = new TraktIds { Tmdb = 27205 } // Same movie
                }
            }
        };

        var handler = CreateMockHandler(new Dictionary<string, string>
        {
            ["users/me/watchlist/movies,shows"] = JsonSerializer.Serialize(watchlistItems),
            ["users/me/collection/movies"] = "[]",
            ["users/me/collection/shows"] = "[]",
            ["users/me/history/movies,shows"] = "[]",
            ["users/me/ratings/movies,shows"] = JsonSerializer.Serialize(ratingItems)
        });

        var client = new HttpClient(handler.Object) { BaseAddress = new Uri("https://api.trakt.tv") };
        var settings = new TraktSettings
        {
            ClientId = "test",
            AccessToken = "valid",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30),
            ImportWatchlist = true,
            ImportCollection = true,
            ImportWatchHistory = true,
            ImportRatings = true
        };

        var importList = CreateImportList(client, settings);
        var result = await importList.FetchAsync();

        Assert.False(result.AnyFailure);
        Assert.Single(result.Items); // Deduplicated
    }

    [Fact]
    public void TraktDeviceCode_DeserializesCorrectly()
    {
        var json = """
        {
            "device_code": "abc123",
            "user_code": "ABCD",
            "verification_url": "https://trakt.tv/activate",
            "expires_in": 600,
            "interval": 5
        }
        """;

        var code = JsonSerializer.Deserialize<TraktDeviceCode>(json);
        Assert.NotNull(code);
        Assert.Equal("abc123", code.DeviceCode);
        Assert.Equal("ABCD", code.UserCode);
        Assert.Equal("https://trakt.tv/activate", code.VerificationUrl);
        Assert.Equal(600, code.ExpiresIn);
        Assert.Equal(5, code.Interval);
    }

    [Fact]
    public void TraktTokenResponse_DeserializesCorrectly()
    {
        var json = """
        {
            "access_token": "at-123",
            "token_type": "Bearer",
            "expires_in": 7776000,
            "refresh_token": "rt-456",
            "scope": "public",
            "created_at": 1706400000
        }
        """;

        var token = JsonSerializer.Deserialize<TraktTokenResponse>(json);
        Assert.NotNull(token);
        Assert.Equal("at-123", token.AccessToken);
        Assert.Equal("rt-456", token.RefreshToken);
        Assert.Equal(7776000, token.ExpiresIn);
    }

    [Fact]
    public void TraktSettings_HasValidToken_ValidWhenNotExpired()
    {
        var settings = new TraktSettings
        {
            AccessToken = "valid",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30)
        };

        Assert.True(settings.HasValidToken);
    }

    [Fact]
    public void TraktSettings_HasValidToken_InvalidWhenExpired()
    {
        var settings = new TraktSettings
        {
            AccessToken = "expired",
            TokenExpiresAt = DateTime.UtcNow.AddDays(-1)
        };

        Assert.False(settings.HasValidToken);
    }

    [Fact]
    public void TraktSettings_HasValidToken_InvalidWhenEmpty()
    {
        var settings = new TraktSettings { AccessToken = "" };
        Assert.False(settings.HasValidToken);
    }

    private static Mock<HttpMessageHandler> CreateMockHandler(Dictionary<string, string> responses)
    {
        var handler = new Mock<HttpMessageHandler>();

        handler.Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .ReturnsAsync((HttpRequestMessage request, CancellationToken _) =>
            {
                var uri = request.RequestUri?.PathAndQuery ?? "";

                foreach (var (key, value) in responses)
                {
                    if (uri.Contains(key))
                    {
                        return new HttpResponseMessage(HttpStatusCode.OK)
                        {
                            Content = new StringContent(value, System.Text.Encoding.UTF8, "application/json")
                        };
                    }
                }

                return new HttpResponseMessage(HttpStatusCode.NotFound);
            });

        return handler;
    }
}
