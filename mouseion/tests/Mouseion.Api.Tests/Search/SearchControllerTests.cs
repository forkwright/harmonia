// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Search;
using Mouseion.Core.Music;
using Mouseion.Core.Search;

namespace Mouseion.Api.Tests.Search;

public class SearchControllerTests
{
    private readonly Mock<ITrackSearchService> _searchService;
    private readonly Mock<IUnifiedSearchService> _unifiedSearchService;
    private readonly SearchController _controller;

    public SearchControllerTests()
    {
        _searchService = new Mock<ITrackSearchService>();
        _unifiedSearchService = new Mock<IUnifiedSearchService>();
        _controller = new SearchController(_searchService.Object, _unifiedSearchService.Object);
    }

    [Fact]
    public async Task Search_ReturnsBadRequest_WhenQueryNull()
    {
        var result = await _controller.Search(null);

        Assert.IsType<BadRequestObjectResult>(result.Result);
    }

    [Fact]
    public async Task Search_ReturnsBadRequest_WhenQueryEmpty()
    {
        var result = await _controller.Search("");

        Assert.IsType<BadRequestObjectResult>(result.Result);
    }

    [Fact]
    public async Task Search_ReturnsBadRequest_WhenQueryWhitespace()
    {
        var result = await _controller.Search("   ");

        Assert.IsType<BadRequestObjectResult>(result.Result);
    }

    [Fact]
    public async Task Search_ReturnsResults_WhenQueryValid()
    {
        var track = new Track
        {
            Id = 1,
            Title = "Bohemian Rhapsody",
            TrackNumber = 11,
            DiscNumber = 1,
            DurationSeconds = 354
        };

        var searchResults = new List<TrackSearchResult>
        {
            new()
            {
                Track = track,
                ArtistName = "Queen",
                AlbumTitle = "A Night at the Opera",
                Genre = "Rock",
                BitDepth = 16,
                DynamicRange = 12,
                Lossless = true,
                RelevanceScore = 0.95
            }
        };

        _searchService.Setup(s => s.SearchAsync("bohemian", 50, It.IsAny<CancellationToken>()))
            .ReturnsAsync(searchResults);

        var result = await _controller.Search("bohemian");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TrackSearchResource>>(okResult.Value);
        Assert.Single(resources);
        Assert.Equal("Bohemian Rhapsody", resources[0].Title);
        Assert.Equal("Queen", resources[0].Artist);
        Assert.Equal("A Night at the Opera", resources[0].Album);
        Assert.True(resources[0].Lossless);
    }

    [Fact]
    public async Task Search_ClampsLimit_WhenBelowMinimum()
    {
        _searchService.Setup(s => s.SearchAsync("test", 50, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<TrackSearchResult>());

        await _controller.Search("test", limit: 0);

        _searchService.Verify(s => s.SearchAsync("test", 50, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Search_ClampsLimit_WhenAboveMaximum()
    {
        _searchService.Setup(s => s.SearchAsync("test", 250, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<TrackSearchResult>());

        await _controller.Search("test", limit: 500);

        _searchService.Verify(s => s.SearchAsync("test", 250, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Search_ReturnsEmptyList_WhenNoResults()
    {
        _searchService.Setup(s => s.SearchAsync("xyznonexistent", 50, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<TrackSearchResult>());

        var result = await _controller.Search("xyznonexistent");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TrackSearchResource>>(okResult.Value);
        Assert.Empty(resources);
    }

    [Fact]
    public async Task SearchAll_ReturnsBadRequest_WhenQueryNull()
    {
        var result = await _controller.SearchAll(null, null);

        Assert.IsType<BadRequestObjectResult>(result.Result);
    }

    [Fact]
    public async Task SearchAll_ReturnsGroupedResults()
    {
        var unified = new UnifiedSearchResult();
        unified.Movies.Add(new SearchHit { Id = 1, MediaType = "movie", Title = "Test Movie", Year = 2024, Score = 100 });
        unified.Tracks.Add(new SearchHit { Id = 2, MediaType = "track", Title = "Test Track", Score = 90 });

        _unifiedSearchService.Setup(s => s.SearchAsync("test", 50, null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(unified);

        var result = await _controller.SearchAll("test", null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var response = Assert.IsType<UnifiedSearchResponse>(okResult.Value);
        Assert.Equal(2, response.TotalResults);
        Assert.Single(response.Movies);
        Assert.Single(response.Tracks);
        Assert.Empty(response.Books);
    }

    [Fact]
    public async Task SearchAll_PassesTypeFilter()
    {
        _unifiedSearchService.Setup(s => s.SearchAsync("test", 50, "movie", It.IsAny<CancellationToken>()))
            .ReturnsAsync(UnifiedSearchResult.Empty);

        await _controller.SearchAll("test", "movie");

        _unifiedSearchService.Verify(s => s.SearchAsync("test", 50, "movie", It.IsAny<CancellationToken>()), Times.Once);
    }
}
