// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Search;
using Mouseion.Core.Music;

namespace Mouseion.Api.Tests.Search;

public class SearchControllerTests
{
    private readonly Mock<ITrackSearchService> _searchService;
    private readonly SearchController _controller;

    public SearchControllerTests()
    {
        _searchService = new Mock<ITrackSearchService>();
        _controller = new SearchController(_searchService.Object);
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
}
