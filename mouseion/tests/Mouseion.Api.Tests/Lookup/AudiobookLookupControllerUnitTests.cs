// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Lookup;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Api.Tests.Lookup;

public class AudiobookLookupControllerUnitTests
{
    private readonly Mock<IProvideAudiobookInfo> _audiobookInfoProviderMock;
    private readonly AudiobookLookupController _controller;

    public AudiobookLookupControllerUnitTests()
    {
        _audiobookInfoProviderMock = new Mock<IProvideAudiobookInfo>();
        _controller = new AudiobookLookupController(_audiobookInfoProviderMock.Object);
    }

    [Fact]
    public async Task Search_WithNoParameters_ReturnsBadRequest()
    {
        var result = await _controller.Search(null, null, null, null);

        var badRequestResult = Assert.IsType<BadRequestObjectResult>(result.Result);
        Assert.NotNull(badRequestResult.Value);
    }

    [Fact]
    public async Task Search_WithTitle_ReturnsResults()
    {
        var audiobooks = new List<Audiobook>
        {
            new Audiobook { Title = "Test Audiobook", Year = 2025, Metadata = new AudiobookMetadata() }
        };
        _audiobookInfoProviderMock.Setup(x => x.SearchByTitleAsync("test", default))
            .ReturnsAsync(audiobooks);

        var result = await _controller.Search("test", null, null, null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<AudiobookLookupResource>>(okResult.Value);
        Assert.Single(resources);
        Assert.Equal("Test Audiobook", resources[0].Title);
    }

    [Fact]
    public async Task Search_WithAuthor_ReturnsResults()
    {
        var audiobooks = new List<Audiobook>
        {
            new Audiobook { Title = "Author Audiobook", Year = 2025, Metadata = new AudiobookMetadata() }
        };
        _audiobookInfoProviderMock.Setup(x => x.SearchByAuthorAsync("author", default))
            .ReturnsAsync(audiobooks);

        var result = await _controller.Search(null, "author", null, null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<AudiobookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task Search_WithNarrator_ReturnsResults()
    {
        var audiobooks = new List<Audiobook>
        {
            new Audiobook { Title = "Narrator Audiobook", Year = 2025, Metadata = new AudiobookMetadata() }
        };
        _audiobookInfoProviderMock.Setup(x => x.SearchByNarratorAsync("narrator", default))
            .ReturnsAsync(audiobooks);

        var result = await _controller.Search(null, null, "narrator", null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<AudiobookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task GetByAsin_WhenAudiobookExists_ReturnsAudiobook()
    {
        var audiobook = new Audiobook { Title = "Found Audiobook", Year = 2025, Metadata = new AudiobookMetadata() };
        _audiobookInfoProviderMock.Setup(x => x.GetByAsinAsync("B0123", default))
            .ReturnsAsync(audiobook);

        var result = await _controller.GetByAsin("B0123");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<AudiobookLookupResource>(okResult.Value);
        Assert.Equal("Found Audiobook", resource.Title);
    }

    [Fact]
    public async Task GetByAsin_WhenNotFound_ReturnsNotFound()
    {
        _audiobookInfoProviderMock.Setup(x => x.GetByAsinAsync("B0999", default))
            .ReturnsAsync(null as Audiobook);

        var result = await _controller.GetByAsin("B0999");

        Assert.IsType<NotFoundObjectResult>(result.Result);
    }

    [Fact]
    public async Task SearchByNarrator_ReturnsResults()
    {
        var audiobooks = new List<Audiobook>
        {
            new Audiobook { Title = "Narrated Audiobook", Year = 2025, Metadata = new AudiobookMetadata() }
        };
        _audiobookInfoProviderMock.Setup(x => x.SearchByNarratorAsync("testnarrator", default))
            .ReturnsAsync(audiobooks);

        var result = await _controller.SearchByNarrator("testnarrator");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<AudiobookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }
}
