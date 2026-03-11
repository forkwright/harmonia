// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using Mouseion.Core.MetadataSource.TVDB;

namespace Mouseion.Core.Tests.MetadataSource;

public class TVDBProxyTests : IDisposable
{
    private readonly Mock<ITVDBClient> _clientMock;
    private readonly IMemoryCache _cache;
    private readonly TVDBProxy _proxy;

    public TVDBProxyTests()
    {
        _clientMock = new Mock<ITVDBClient>();
        _cache = new MemoryCache(new MemoryCacheOptions());
        _proxy = new TVDBProxy(
            _clientMock.Object,
            _cache,
            NullLogger<TVDBProxy>.Instance);
    }

    public void Dispose()
    {
        _cache.Dispose();
    }

    #region GetSeriesByTvdbIdAsync

    [Fact]
    public async Task GetSeriesByTvdbIdAsync_should_return_series_for_valid_response()
    {
        var json = """
        {
            "status": "success",
            "data": {
                "id": 81189,
                "name": "Breaking Bad",
                "overview": "A high school chemistry teacher diagnosed with lung cancer.",
                "status": { "name": "Ended" },
                "firstAired": "2008-01-20",
                "averageRuntime": 47,
                "originalNetwork": { "name": "AMC" },
                "genres": [
                    { "name": "Drama" },
                    { "name": "Thriller" }
                ],
                "remoteIds": [
                    { "sourceName": "IMDB", "id": "tt0903747" },
                    { "sourceName": "TheMovieDB", "id": "1396" }
                ]
            }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/81189/extended", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.GetSeriesByTvdbIdAsync(81189);

        Assert.NotNull(result);
        Assert.Equal(81189, result.TvdbId);
        Assert.Equal("Breaking Bad", result.Title);
        Assert.Contains("high school chemistry", result.Overview);
        Assert.Equal("Ended", result.Status);
        Assert.Equal(2008, result.Year);
        Assert.Equal(47, result.Runtime);
        Assert.Equal("AMC", result.Network);
        Assert.Contains("Drama", result.Genres);
        Assert.Contains("Thriller", result.Genres);
        Assert.Equal("tt0903747", result.ImdbId);
        Assert.Equal(1396, result.TmdbId);
    }

    [Fact]
    public async Task GetSeriesByTvdbIdAsync_should_return_null_when_api_returns_null()
    {
        _clientMock.Setup(c => c.GetAsync("series/12345/extended", It.IsAny<CancellationToken>()))
            .ReturnsAsync((string?)null);

        var result = await _proxy.GetSeriesByTvdbIdAsync(12345);

        Assert.Null(result);
    }

    [Fact]
    public async Task GetSeriesByTvdbIdAsync_should_return_null_for_invalid_json()
    {
        _clientMock.Setup(c => c.GetAsync("series/12345/extended", It.IsAny<CancellationToken>()))
            .ReturnsAsync("{ invalid json }");

        var result = await _proxy.GetSeriesByTvdbIdAsync(12345);

        Assert.Null(result);
    }

    [Fact]
    public async Task GetSeriesByTvdbIdAsync_should_cache_result()
    {
        var json = """
        {
            "data": {
                "id": 81189,
                "name": "Breaking Bad",
                "firstAired": "2008-01-20"
            }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/81189/extended", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        // First call
        await _proxy.GetSeriesByTvdbIdAsync(81189);

        // Second call should use cache
        await _proxy.GetSeriesByTvdbIdAsync(81189);

        _clientMock.Verify(c => c.GetAsync("series/81189/extended", It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetSeriesByTvdbIdAsync_should_handle_missing_optional_fields()
    {
        var json = """
        {
            "data": {
                "id": 12345,
                "name": "Test Series"
            }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/12345/extended", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.GetSeriesByTvdbIdAsync(12345);

        Assert.NotNull(result);
        Assert.Equal("Test Series", result.Title);
        Assert.Null(result.Overview);
        Assert.Equal("Continuing", result.Status); // default
        Assert.Empty(result.Genres);
    }

    #endregion

    #region GetEpisodesBySeriesIdAsync

    [Fact]
    public async Task GetEpisodesBySeriesIdAsync_should_return_episodes()
    {
        var json = """
        {
            "data": {
                "episodes": [
                    {
                        "seasonNumber": 1,
                        "number": 1,
                        "absoluteNumber": 1,
                        "name": "Pilot",
                        "overview": "When an unassuming high school chemistry teacher...",
                        "aired": "2008-01-20"
                    },
                    {
                        "seasonNumber": 1,
                        "number": 2,
                        "name": "Cat's in the Bag...",
                        "aired": "2008-01-27"
                    }
                ]
            },
            "links": {
                "prev": null,
                "self": "https://api4.thetvdb.com/v4/series/81189/episodes/default?page=0",
                "next": null
            }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/81189/episodes/default?page=0", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.GetEpisodesBySeriesIdAsync(81189);

        Assert.Equal(2, result.Count);

        var pilot = result[0];
        Assert.Equal(1, pilot.SeasonNumber);
        Assert.Equal(1, pilot.EpisodeNumber);
        Assert.Equal(1, pilot.AbsoluteEpisodeNumber);
        Assert.Equal("Pilot", pilot.Title);
        Assert.NotNull(pilot.Overview);
        Assert.Equal(new DateTime(2008, 1, 20), pilot.AirDate);
    }

    [Fact]
    public async Task GetEpisodesBySeriesIdAsync_should_handle_pagination()
    {
        var page0 = """
        {
            "data": {
                "episodes": [
                    { "seasonNumber": 1, "number": 1, "name": "Ep1" },
                    { "seasonNumber": 1, "number": 2, "name": "Ep2" }
                ]
            },
            "links": { "next": "page1" }
        }
        """;

        var page1 = """
        {
            "data": {
                "episodes": [
                    { "seasonNumber": 1, "number": 3, "name": "Ep3" }
                ]
            },
            "links": { "next": null }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/81189/episodes/default?page=0", It.IsAny<CancellationToken>()))
            .ReturnsAsync(page0);
        _clientMock.Setup(c => c.GetAsync("series/81189/episodes/default?page=1", It.IsAny<CancellationToken>()))
            .ReturnsAsync(page1);

        var result = await _proxy.GetEpisodesBySeriesIdAsync(81189);

        Assert.Equal(3, result.Count);
        Assert.Equal("Ep1", result[0].Title);
        Assert.Equal("Ep3", result[2].Title);
    }

    [Fact]
    public async Task GetEpisodesBySeriesIdAsync_should_skip_specials_with_episode_zero()
    {
        var json = """
        {
            "data": {
                "episodes": [
                    { "seasonNumber": 0, "number": 0, "name": "Behind the Scenes" },
                    { "seasonNumber": 1, "number": 1, "name": "Pilot" }
                ]
            },
            "links": { "next": null }
        }
        """;

        _clientMock.Setup(c => c.GetAsync("series/81189/episodes/default?page=0", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.GetEpisodesBySeriesIdAsync(81189);

        Assert.Single(result);
        Assert.Equal("Pilot", result[0].Title);
    }

    [Fact]
    public async Task GetEpisodesBySeriesIdAsync_should_return_empty_list_on_error()
    {
        _clientMock.Setup(c => c.GetAsync("series/12345/episodes/default?page=0", It.IsAny<CancellationToken>()))
            .ReturnsAsync((string?)null);

        var result = await _proxy.GetEpisodesBySeriesIdAsync(12345);

        Assert.Empty(result);
    }

    #endregion

    #region SearchSeriesAsync

    [Fact]
    public async Task SearchSeriesAsync_should_return_search_results()
    {
        var json = """
        {
            "data": [
                {
                    "type": "series",
                    "tvdb_id": "81189",
                    "name": "Breaking Bad",
                    "year": "2008",
                    "status": "Ended",
                    "network": "AMC",
                    "overview": "A chemistry teacher...",
                    "image_url": "https://artworks.thetvdb.com/banners/posters/81189-1.jpg"
                },
                {
                    "type": "series",
                    "tvdb_id": "12345",
                    "name": "Breaking Bad: The Documentary",
                    "year": "2013"
                }
            ]
        }
        """;

        _clientMock.Setup(c => c.GetAsync("search?query=Breaking%20Bad&type=series", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.SearchSeriesAsync("Breaking Bad");

        Assert.Equal(2, result.Count);

        var first = result[0];
        Assert.Equal(81189, first.TvdbId);
        Assert.Equal("Breaking Bad", first.Title);
        Assert.Equal(2008, first.Year);
        Assert.Equal("AMC", first.Network);
        Assert.Contains("chemistry teacher", first.Overview);
        Assert.Single(first.Images);
    }

    [Fact]
    public async Task SearchSeriesAsync_should_filter_non_series_results()
    {
        var json = """
        {
            "data": [
                { "type": "series", "tvdb_id": "81189", "name": "Breaking Bad" },
                { "type": "movie", "tvdb_id": "99999", "name": "Breaking Bad Movie" },
                { "type": "person", "tvdb_id": "88888", "name": "Bryan Cranston" }
            ]
        }
        """;

        _clientMock.Setup(c => c.GetAsync("search?query=test&type=series", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        var result = await _proxy.SearchSeriesAsync("test");

        Assert.Single(result);
        Assert.Equal("Breaking Bad", result[0].Title);
    }

    [Fact]
    public async Task SearchSeriesAsync_should_return_empty_for_null_query()
    {
        var result = await _proxy.SearchSeriesAsync(null!);

        Assert.Empty(result);
        _clientMock.Verify(c => c.GetAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task SearchSeriesAsync_should_return_empty_for_whitespace_query()
    {
        var result = await _proxy.SearchSeriesAsync("   ");

        Assert.Empty(result);
        _clientMock.Verify(c => c.GetAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task SearchSeriesAsync_should_cache_results()
    {
        var json = """{ "data": [{ "type": "series", "tvdb_id": "81189", "name": "Test" }] }""";

        _clientMock.Setup(c => c.GetAsync("search?query=test&type=series", It.IsAny<CancellationToken>()))
            .ReturnsAsync(json);

        await _proxy.SearchSeriesAsync("test");
        await _proxy.SearchSeriesAsync("test");

        _clientMock.Verify(c => c.GetAsync("search?query=test&type=series", It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task SearchSeriesAsync_should_url_encode_query()
    {
        _clientMock.Setup(c => c.GetAsync("search?query=The%20Office%20%28US%29&type=series", It.IsAny<CancellationToken>()))
            .ReturnsAsync("""{ "data": [] }""");

        await _proxy.SearchSeriesAsync("The Office (US)");

        _clientMock.Verify(c => c.GetAsync("search?query=The%20Office%20%28US%29&type=series", It.IsAny<CancellationToken>()), Times.Once);
    }

    #endregion
}
