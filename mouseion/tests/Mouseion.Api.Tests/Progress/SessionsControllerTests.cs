// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Progress;
using Mouseion.Core.Books;
using Mouseion.Core.MediaItems;
using Mouseion.Core.Progress;

namespace Mouseion.Api.Tests.Progress;

public class SessionsControllerTests
{
    private readonly Mock<IPlaybackSessionRepository> _sessionRepo;
    private readonly Mock<IMediaItemRepository> _mediaItemRepo;
    private readonly SessionsController _controller;

    public SessionsControllerTests()
    {
        _sessionRepo = new Mock<IPlaybackSessionRepository>();
        _mediaItemRepo = new Mock<IMediaItemRepository>();
        _controller = new SessionsController(_sessionRepo.Object, _mediaItemRepo.Object);
    }

    [Fact]
    public async Task GetSessions_ReturnsRecentSessions_ByDefault()
    {
        var sessions = new List<PlaybackSession>
        {
            new() { SessionId = "sess-1", MediaItemId = 1, UserId = "default", StartedAt = DateTime.UtcNow },
            new() { SessionId = "sess-2", MediaItemId = 2, UserId = "default", StartedAt = DateTime.UtcNow }
        };

        _sessionRepo.Setup(r => r.GetRecentSessionsAsync("default", 100, It.IsAny<CancellationToken>()))
            .ReturnsAsync(sessions);

        var result = await _controller.GetSessions();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<PlaybackSessionResource>>(okResult.Value);
        Assert.Equal(2, resources.Count);
    }

    [Fact]
    public async Task GetSessions_ReturnsActiveSessions_WhenActiveOnlyTrue()
    {
        var sessions = new List<PlaybackSession>
        {
            new() { SessionId = "active-1", IsActive = true }
        };

        _sessionRepo.Setup(r => r.GetActiveSessionsAsync("default", It.IsAny<CancellationToken>()))
            .ReturnsAsync(sessions);

        var result = await _controller.GetSessions(activeOnly: true);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<PlaybackSessionResource>>(okResult.Value);
        Assert.Single(resources);
        _sessionRepo.Verify(r => r.GetActiveSessionsAsync("default", It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetSession_ReturnsOk_WhenFound()
    {
        var session = new PlaybackSession
        {
            Id = 1,
            SessionId = "sess-abc",
            MediaItemId = 42,
            UserId = "default",
            DeviceName = "iPhone",
            DeviceType = "mobile",
            StartedAt = DateTime.UtcNow,
            IsActive = true
        };

        _sessionRepo.Setup(r => r.GetBySessionIdAsync("sess-abc", It.IsAny<CancellationToken>()))
            .ReturnsAsync(session);

        var result = await _controller.GetSession("sess-abc");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<PlaybackSessionResource>(okResult.Value);
        Assert.Equal("sess-abc", resource.SessionId);
        Assert.Equal("iPhone", resource.DeviceName);
        Assert.True(resource.IsActive);
    }

    [Fact]
    public async Task GetSession_ReturnsNotFound_WhenMissing()
    {
        _sessionRepo.Setup(r => r.GetBySessionIdAsync("nonexistent", It.IsAny<CancellationToken>()))
            .ReturnsAsync((PlaybackSession?)null);

        var result = await _controller.GetSession("nonexistent");

        Assert.IsType<NotFoundObjectResult>(result.Result);
    }

    [Fact]
    public async Task GetSessionsByMediaItem_ReturnsSessions()
    {
        var sessions = new List<PlaybackSession>
        {
            new() { SessionId = "s1", MediaItemId = 10 },
            new() { SessionId = "s2", MediaItemId = 10 }
        };

        _sessionRepo.Setup(r => r.GetByMediaItemIdAsync(10, "default", It.IsAny<CancellationToken>()))
            .ReturnsAsync(sessions);

        var result = await _controller.GetSessionsByMediaItem(10);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<PlaybackSessionResource>>(okResult.Value);
        Assert.Equal(2, resources.Count);
    }

    [Fact]
    public async Task StartSession_ReturnsCreated_WhenMediaItemExists()
    {
        var mediaItem = new Book { Title = "Test" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        _sessionRepo.Setup(r => r.InsertAsync(It.IsAny<PlaybackSession>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((PlaybackSession s, CancellationToken _) =>
            {
                s.Id = 1;
                return s;
            });

        var request = new StartSessionRequest
        {
            MediaItemId = 1,
            DeviceName = "Pixel 9",
            DeviceType = "mobile",
            StartPositionMs = 5000
        };

        var result = await _controller.StartSession(request);

        var createdResult = Assert.IsType<CreatedAtActionResult>(result.Result);
        var resource = Assert.IsType<PlaybackSessionResource>(createdResult.Value);
        Assert.Equal(1, resource.MediaItemId);
        Assert.Equal("Pixel 9", resource.DeviceName);
        Assert.Equal(5000, resource.StartPositionMs);
        Assert.True(resource.IsActive);
    }

    [Fact]
    public async Task StartSession_ReturnsNotFound_WhenMediaItemMissing()
    {
        _mediaItemRepo.Setup(r => r.FindByIdAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((MediaItem?)null);

        var request = new StartSessionRequest { MediaItemId = 999 };

        var result = await _controller.StartSession(request);

        Assert.IsType<NotFoundObjectResult>(result.Result);
        _sessionRepo.Verify(r => r.InsertAsync(It.IsAny<PlaybackSession>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task StartSession_DefaultsUserAndDevice_WhenNotProvided()
    {
        var mediaItem = new Book { Title = "Test" };
        _mediaItemRepo.Setup(r => r.FindByIdAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(mediaItem);

        _sessionRepo.Setup(r => r.InsertAsync(It.IsAny<PlaybackSession>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((PlaybackSession s, CancellationToken _) => { s.Id = 1; return s; });

        var request = new StartSessionRequest { MediaItemId = 1 };

        await _controller.StartSession(request);

        _sessionRepo.Verify(r => r.InsertAsync(
            It.Is<PlaybackSession>(s =>
                s.UserId == "default" &&
                s.DeviceName == "Unknown Device" &&
                s.DeviceType == "Unknown"),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateSession_EndsSession_WhenEndSessionTrue()
    {
        var session = new PlaybackSession { SessionId = "sess-end", IsActive = true };

        _sessionRepo.Setup(r => r.GetBySessionIdAsync("sess-end", It.IsAny<CancellationToken>()))
            .ReturnsAsync(session);
        _sessionRepo.Setup(r => r.EndSessionAsync("sess-end", 300000, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var request = new UpdateSessionRequest { EndSession = true, EndPositionMs = 300000 };

        await _controller.UpdateSession("sess-end", request);

        _sessionRepo.Verify(r => r.EndSessionAsync("sess-end", 300000, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateSession_ReturnsNotFound_WhenSessionMissing()
    {
        _sessionRepo.Setup(r => r.GetBySessionIdAsync("gone", It.IsAny<CancellationToken>()))
            .ReturnsAsync((PlaybackSession?)null);

        var request = new UpdateSessionRequest { EndSession = true };

        var result = await _controller.UpdateSession("gone", request);

        Assert.IsType<NotFoundObjectResult>(result.Result);
    }

    [Fact]
    public async Task DeleteSession_ReturnsNoContent_WhenExists()
    {
        var session = new PlaybackSession { Id = 5, SessionId = "sess-del" };

        _sessionRepo.Setup(r => r.GetBySessionIdAsync("sess-del", It.IsAny<CancellationToken>()))
            .ReturnsAsync(session);
        _sessionRepo.Setup(r => r.DeleteAsync(5, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var result = await _controller.DeleteSession("sess-del");

        Assert.IsType<NoContentResult>(result);
        _sessionRepo.Verify(r => r.DeleteAsync(5, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteSession_ReturnsNotFound_WhenMissing()
    {
        _sessionRepo.Setup(r => r.GetBySessionIdAsync("nope", It.IsAny<CancellationToken>()))
            .ReturnsAsync((PlaybackSession?)null);

        var result = await _controller.DeleteSession("nope");

        Assert.IsType<NotFoundObjectResult>(result);
    }
}
