// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Progress;
using Mouseion.Core.Books;
using Mouseion.Core.MediaFiles;
using Mouseion.Core.MediaItems;
using Mouseion.Core.Progress;

namespace Mouseion.Api.Tests.Progress;

public class ContinueWatchingControllerTests
{
    private readonly Mock<IMediaProgressRepository> _progressRepo;
    private readonly Mock<IMediaItemRepository> _mediaItemRepo;
    private readonly Mock<IMediaFileRepository> _mediaFileRepo;
    private readonly ContinueWatchingController _controller;

    public ContinueWatchingControllerTests()
    {
        _progressRepo = new Mock<IMediaProgressRepository>();
        _mediaItemRepo = new Mock<IMediaItemRepository>();
        _mediaFileRepo = new Mock<IMediaFileRepository>();
        _controller = new ContinueWatchingController(
            _progressRepo.Object,
            _mediaItemRepo.Object,
            _mediaFileRepo.Object);
    }

    [Fact]
    public async Task GetContinue_ReturnsOk_WithEmptyList_WhenNoProgress()
    {
        _progressRepo.Setup(r => r.GetInProgressAsync("default", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaProgress>());

        var result = await _controller.GetContinue();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var items = Assert.IsType<List<ContinueResource>>(okResult.Value);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetContinue_ReturnsItems_WithMediaDetails()
    {
        var progressList = new List<MediaProgress>
        {
            new()
            {
                MediaItemId = 1,
                PositionMs = 120000,
                TotalDurationMs = 360000,
                PercentComplete = 33.33m,
                LastPlayedAt = DateTime.UtcNow
            }
        };

        var book = new Book { Title = "Test Book" };
        var mediaFile = new MediaFile { Id = 10, Path = "/media/test.m4b" };

        _progressRepo.Setup(r => r.GetInProgressAsync("default", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(progressList);
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(book);
        _mediaFileRepo.Setup(r => r.GetByMediaItemIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaFile> { mediaFile });

        var result = await _controller.GetContinue();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var items = Assert.IsType<List<ContinueResource>>(okResult.Value);
        Assert.Single(items);
        Assert.Equal(1, items[0].MediaItemId);
        Assert.Equal(120000, items[0].PositionMs);
        Assert.Equal(33.33m, items[0].PercentComplete);
        Assert.Equal(10, items[0].MediaFileId);
    }

    [Fact]
    public async Task GetContinue_SkipsItems_WhenMediaItemNotFound()
    {
        var progressList = new List<MediaProgress>
        {
            new() { MediaItemId = 1 },
            new() { MediaItemId = 2 }
        };

        _progressRepo.Setup(r => r.GetInProgressAsync("default", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(progressList);
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MediaItem?)null);
        _mediaItemRepo.Setup(r => r.FindByIdAsync(2, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Book { Title = "Found Book" });
        _mediaFileRepo.Setup(r => r.GetByMediaItemIdAsync(2, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaFile>());

        var result = await _controller.GetContinue();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var items = Assert.IsType<List<ContinueResource>>(okResult.Value);
        Assert.Single(items);
    }

    [Fact]
    public async Task GetContinue_HandlesNoMediaFiles()
    {
        var progressList = new List<MediaProgress>
        {
            new() { MediaItemId = 5, PositionMs = 1000 }
        };

        _progressRepo.Setup(r => r.GetInProgressAsync("default", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(progressList);
        _mediaItemRepo.Setup(r => r.FindByIdAsync(5, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Book { Title = "No Files Book" });
        _mediaFileRepo.Setup(r => r.GetByMediaItemIdAsync(5, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaFile>());

        var result = await _controller.GetContinue();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var items = Assert.IsType<List<ContinueResource>>(okResult.Value);
        Assert.Single(items);
        Assert.Null(items[0].MediaFileId);
    }

    [Fact]
    public async Task GetContinue_RespectsCustomLimit()
    {
        _progressRepo.Setup(r => r.GetInProgressAsync("default", 5, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaProgress>());

        await _controller.GetContinue(limit: 5);

        _progressRepo.Verify(r => r.GetInProgressAsync("default", 5, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetContinue_RespectsCustomUserId()
    {
        _progressRepo.Setup(r => r.GetInProgressAsync("user-123", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaProgress>());

        await _controller.GetContinue("user-123");

        _progressRepo.Verify(r => r.GetInProgressAsync("user-123", 20, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetContinue_BuildsCoverUrl_FromMediaItemId()
    {
        var progressList = new List<MediaProgress>
        {
            new() { MediaItemId = 77 }
        };

        _progressRepo.Setup(r => r.GetInProgressAsync("default", 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync(progressList);
        _mediaItemRepo.Setup(r => r.FindByIdAsync(77, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Book { Title = "Cover Test" });
        _mediaFileRepo.Setup(r => r.GetByMediaItemIdAsync(77, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<MediaFile>());

        var result = await _controller.GetContinue();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var items = Assert.IsType<List<ContinueResource>>(okResult.Value);
        Assert.Equal("/api/v3/mediacover/77/poster", items[0].CoverUrl);
    }
}
