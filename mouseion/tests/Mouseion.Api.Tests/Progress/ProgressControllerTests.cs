// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Progress;
using Mouseion.Core.Books;
using Mouseion.Core.MediaItems;
using Mouseion.Core.Progress;

namespace Mouseion.Api.Tests.Progress;

public class ProgressControllerTests
{
    private readonly Mock<IMediaProgressRepository> _progressRepo;
    private readonly Mock<IMediaItemRepository> _mediaItemRepo;
    private readonly ProgressController _controller;

    public ProgressControllerTests()
    {
        _progressRepo = new Mock<IMediaProgressRepository>();
        _mediaItemRepo = new Mock<IMediaItemRepository>();
        _controller = new ProgressController(_progressRepo.Object, _mediaItemRepo.Object);
    }

    [Fact]
    public async Task GetProgress_ReturnsOk_WhenProgressExists()
    {
        var progress = new MediaProgress
        {
            Id = 1,
            MediaItemId = 42,
            UserId = "default",
            PositionMs = 120000,
            TotalDurationMs = 360000,
            PercentComplete = 33.33m,
            LastPlayedAt = DateTime.UtcNow,
            IsComplete = false
        };

        _progressRepo.Setup(r => r.GetByMediaItemIdAsync(42, "default", It.IsAny<CancellationToken>()))
            .ReturnsAsync(progress);

        var result = await _controller.GetProgress(42);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<MediaProgressResource>(okResult.Value);
        Assert.Equal(42, resource.MediaItemId);
        Assert.Equal(120000, resource.PositionMs);
        Assert.Equal(33.33m, resource.PercentComplete);
    }

    [Fact]
    public async Task GetProgress_ReturnsNotFound_WhenNoProgress()
    {
        _progressRepo.Setup(r => r.GetByMediaItemIdAsync(99, "default", It.IsAny<CancellationToken>()))
            .ReturnsAsync((MediaProgress?)null);

        var result = await _controller.GetProgress(99);

        Assert.IsType<NotFoundObjectResult>(result.Result);
    }

    [Fact]
    public async Task GetProgress_UsesProvidedUserId()
    {
        var progress = new MediaProgress
        {
            Id = 2,
            MediaItemId = 10,
            UserId = "user-abc",
            PositionMs = 5000
        };

        _progressRepo.Setup(r => r.GetByMediaItemIdAsync(10, "user-abc", It.IsAny<CancellationToken>()))
            .ReturnsAsync(progress);

        var result = await _controller.GetProgress(10, "user-abc");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<MediaProgressResource>(okResult.Value);
        Assert.Equal("user-abc", resource.UserId);
    }

    [Fact]
    public async Task UpdateProgress_ReturnsOk_WhenMediaItemExists()
    {
        var mediaItem = new Book { Title = "Test Book" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        var request = new UpdateProgressRequest
        {
            MediaItemId = 1,
            PositionMs = 60000,
            TotalDurationMs = 180000,
            IsComplete = false
        };

        var result = await _controller.UpdateProgress(request);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<MediaProgressResource>(okResult.Value);
        Assert.Equal(1, resource.MediaItemId);
        Assert.Equal(60000, resource.PositionMs);
        Assert.Equal(33.33m, resource.PercentComplete);
        _progressRepo.Verify(r => r.UpsertAsync(It.IsAny<MediaProgress>(), It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateProgress_ReturnsNotFound_WhenMediaItemMissing()
    {
        _mediaItemRepo.Setup(r => r.FindByIdAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MediaItem?)null);

        var request = new UpdateProgressRequest
        {
            MediaItemId = 999,
            PositionMs = 1000,
            TotalDurationMs = 5000
        };

        var result = await _controller.UpdateProgress(request);

        Assert.IsType<NotFoundObjectResult>(result.Result);
        _progressRepo.Verify(r => r.UpsertAsync(It.IsAny<MediaProgress>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task UpdateProgress_CalculatesPercentComplete_Correctly()
    {
        var mediaItem = new Book { Title = "Test" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        var request = new UpdateProgressRequest
        {
            MediaItemId = 1,
            PositionMs = 90000,
            TotalDurationMs = 360000
        };

        var result = await _controller.UpdateProgress(request);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<MediaProgressResource>(okResult.Value);
        Assert.Equal(25.00m, resource.PercentComplete);
    }

    [Fact]
    public async Task UpdateProgress_HandlesZeroDuration()
    {
        var mediaItem = new Book { Title = "Test" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        var request = new UpdateProgressRequest
        {
            MediaItemId = 1,
            PositionMs = 5000,
            TotalDurationMs = 0
        };

        var result = await _controller.UpdateProgress(request);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<MediaProgressResource>(okResult.Value);
        Assert.Equal(0m, resource.PercentComplete);
    }

    [Fact]
    public async Task UpdateProgress_DefaultsUserId_WhenNotProvided()
    {
        var mediaItem = new Book { Title = "Test" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        var request = new UpdateProgressRequest
        {
            MediaItemId = 1,
            PositionMs = 1000,
            TotalDurationMs = 5000
        };

        await _controller.UpdateProgress(request);

        _progressRepo.Verify(r => r.UpsertAsync(
            It.Is<MediaProgress>(p => p.UserId == "default"),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteProgress_ReturnsNoContent()
    {
        _progressRepo.Setup(r => r.DeleteByMediaItemIdAsync(42, "default", It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var result = await _controller.DeleteProgress(42);

        Assert.IsType<NoContentResult>(result);
        _progressRepo.Verify(r => r.DeleteByMediaItemIdAsync(42, "default", It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteProgress_UsesProvidedUserId()
    {
        _progressRepo.Setup(r => r.DeleteByMediaItemIdAsync(42, "user-xyz", It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        await _controller.DeleteProgress(42, "user-xyz");

        _progressRepo.Verify(r => r.DeleteByMediaItemIdAsync(42, "user-xyz", It.IsAny<CancellationToken>()), Times.Once);
    }
}
