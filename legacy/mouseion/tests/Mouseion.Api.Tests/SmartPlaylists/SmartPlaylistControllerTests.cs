// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Api.SmartPlaylists;
using Mouseion.Core.SmartPlaylists;
using Xunit;

namespace Mouseion.Api.Tests.SmartPlaylists;

public class SmartPlaylistControllerTests
{
    private readonly Mock<ISmartPlaylistService> _serviceMock;
    private readonly SmartPlaylistController _controller;

    public SmartPlaylistControllerTests()
    {
        _serviceMock = new Mock<ISmartPlaylistService>();
        var loggerMock = new Mock<ILogger<SmartPlaylistController>>();
        _controller = new SmartPlaylistController(_serviceMock.Object, loggerMock.Object);
    }

    [Fact]
    public async Task List_ReturnsOkWithPlaylists()
    {
        var playlists = new List<SmartPlaylist>
        {
            new() { Id = 1, Name = "Rock" },
            new() { Id = 2, Name = "Jazz" }
        };
        _serviceMock.Setup(s => s.GetAllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlists);

        var result = await _controller.List(CancellationToken.None);

        var ok = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<SmartPlaylistResource>>(ok.Value);
        Assert.Equal(2, resources.Count);
    }

    [Fact]
    public async Task Get_WithValidId_ReturnsOkWithTracks()
    {
        var playlist = new SmartPlaylist { Id = 1, Name = "Rock", FilterRequestJson = "{}" };
        var tracks = new List<SmartPlaylistTrack>
        {
            new() { SmartPlaylistId = 1, TrackId = 10, Position = 0 },
            new() { SmartPlaylistId = 1, TrackId = 20, Position = 1 }
        };
        _serviceMock.Setup(s => s.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlist);
        _serviceMock.Setup(s => s.GetTracksAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(tracks);

        var result = await _controller.Get(1, CancellationToken.None);

        var ok = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<SmartPlaylistResource>(ok.Value);
        Assert.Equal("Rock", resource.Name);
        Assert.NotNull(resource.Tracks);
        Assert.Equal(2, resource.Tracks.Count);
    }

    [Fact]
    public async Task Get_WithInvalidId_ReturnsNotFound()
    {
        _serviceMock.Setup(s => s.GetAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        var result = await _controller.Get(999, CancellationToken.None);

        Assert.IsType<NotFoundResult>(result.Result);
    }

    [Fact]
    public async Task Create_ReturnsCreatedAtAction()
    {
        var created = new SmartPlaylist
        {
            Id = 1,
            Name = "New Playlist",
            FilterRequestJson = "{}",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };
        _serviceMock.Setup(s => s.CreateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(created);

        var resource = new SmartPlaylistResource { Name = "New Playlist", FilterRequestJson = "{}" };
        var result = await _controller.Create(resource, CancellationToken.None);

        var createdResult = Assert.IsType<CreatedAtActionResult>(result.Result);
        Assert.Equal(nameof(SmartPlaylistController.Get), createdResult.ActionName);
        var returnedResource = Assert.IsType<SmartPlaylistResource>(createdResult.Value);
        Assert.Equal(1, returnedResource.Id);
        Assert.Equal("New Playlist", returnedResource.Name);
    }

    [Fact]
    public async Task Update_WithValidId_ReturnsOk()
    {
        var existing = new SmartPlaylist
        {
            Id = 1,
            Name = "Original",
            FilterRequestJson = "{}",
            CreatedAt = DateTime.UtcNow.AddDays(-1)
        };
        _serviceMock.Setup(s => s.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existing);
        _serviceMock.Setup(s => s.UpdateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist p, CancellationToken _) => p);

        var resource = new SmartPlaylistResource { Name = "Updated", FilterRequestJson = "{}" };
        var result = await _controller.Update(1, resource, CancellationToken.None);

        var ok = Assert.IsType<OkObjectResult>(result.Result);
        var returned = Assert.IsType<SmartPlaylistResource>(ok.Value);
        Assert.Equal("Updated", returned.Name);
    }

    [Fact]
    public async Task Update_WithInvalidId_ReturnsNotFound()
    {
        _serviceMock.Setup(s => s.GetAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        var resource = new SmartPlaylistResource { Name = "Test", FilterRequestJson = "{}" };
        var result = await _controller.Update(999, resource, CancellationToken.None);

        Assert.IsType<NotFoundResult>(result.Result);
    }

    [Fact]
    public async Task Delete_WithValidId_ReturnsNoContent()
    {
        var existing = new SmartPlaylist { Id = 1, Name = "To Delete" };
        _serviceMock.Setup(s => s.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existing);

        var result = await _controller.Delete(1, CancellationToken.None);

        Assert.IsType<NoContentResult>(result);
        _serviceMock.Verify(s => s.DeleteAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Delete_WithInvalidId_ReturnsNotFound()
    {
        _serviceMock.Setup(s => s.GetAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        var result = await _controller.Delete(999, CancellationToken.None);

        Assert.IsType<NotFoundResult>(result);
    }

    [Fact]
    public async Task Refresh_WithValidId_ReturnsRefreshedPlaylist()
    {
        var existing = new SmartPlaylist { Id = 1, Name = "Rock" };
        var refreshed = new SmartPlaylist
        {
            Id = 1,
            Name = "Rock",
            TrackCount = 5,
            LastRefreshed = DateTime.UtcNow
        };
        var tracks = Enumerable.Range(0, 5)
            .Select(i => new SmartPlaylistTrack { SmartPlaylistId = 1, TrackId = i + 1, Position = i })
            .ToList();

        _serviceMock.Setup(s => s.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existing);
        _serviceMock.Setup(s => s.RefreshAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(refreshed);
        _serviceMock.Setup(s => s.GetTracksAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(tracks);

        var result = await _controller.Refresh(1, CancellationToken.None);

        var ok = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<SmartPlaylistResource>(ok.Value);
        Assert.Equal(5, resource.TrackCount);
        Assert.NotNull(resource.Tracks);
        Assert.Equal(5, resource.Tracks.Count);
    }

    [Fact]
    public async Task Refresh_WithInvalidId_ReturnsNotFound()
    {
        _serviceMock.Setup(s => s.GetAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        var result = await _controller.Refresh(999, CancellationToken.None);

        Assert.IsType<NotFoundResult>(result.Result);
    }

    [Fact]
    public async Task Update_PreservesCreatedAt()
    {
        var createdAt = new DateTime(2025, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        var existing = new SmartPlaylist
        {
            Id = 1,
            Name = "Original",
            FilterRequestJson = "{}",
            CreatedAt = createdAt
        };
        _serviceMock.Setup(s => s.GetAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(existing);
        _serviceMock.Setup(s => s.UpdateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist p, CancellationToken _) => p);

        var resource = new SmartPlaylistResource
        {
            Name = "Updated",
            FilterRequestJson = "{}",
            CreatedAt = DateTime.UtcNow // Client tries to override
        };
        var result = await _controller.Update(1, resource, CancellationToken.None);

        _serviceMock.Verify(s => s.UpdateAsync(
            It.Is<SmartPlaylist>(p => p.CreatedAt == createdAt),
            It.IsAny<CancellationToken>()), Times.Once);
    }
}

public class SmartPlaylistResourceValidatorTests
{
    private readonly SmartPlaylistResourceValidator _validator = new();

    [Fact]
    public void Valid_Resource_Passes()
    {
        var resource = new SmartPlaylistResource
        {
            Name = "Test",
            FilterRequestJson = "{}"
        };

        var result = _validator.Validate(resource);

        Assert.True(result.IsValid);
    }

    [Fact]
    public void EmptyName_Fails()
    {
        var resource = new SmartPlaylistResource
        {
            Name = "",
            FilterRequestJson = "{}"
        };

        var result = _validator.Validate(resource);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.PropertyName == "Name");
    }

    [Fact]
    public void NameTooLong_Fails()
    {
        var resource = new SmartPlaylistResource
        {
            Name = new string('x', 201),
            FilterRequestJson = "{}"
        };

        var result = _validator.Validate(resource);

        Assert.False(result.IsValid);
    }

    [Fact]
    public void InvalidJson_Fails()
    {
        var resource = new SmartPlaylistResource
        {
            Name = "Test",
            FilterRequestJson = "not json"
        };

        var result = _validator.Validate(resource);

        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.PropertyName == "FilterRequestJson");
    }

    [Fact]
    public void EmptyFilterJson_Fails()
    {
        var resource = new SmartPlaylistResource
        {
            Name = "Test",
            FilterRequestJson = ""
        };

        var result = _validator.Validate(resource);

        Assert.False(result.IsValid);
    }

    [Fact]
    public void ComplexFilterJson_Passes()
    {
        var resource = new SmartPlaylistResource
        {
            Name = "Rock Playlist",
            FilterRequestJson = "{\"Conditions\":[{\"Field\":\"Genre\",\"Operator\":0,\"Value\":\"Rock\"},{\"Field\":\"Year\",\"Operator\":4,\"Value\":\"2020\"}],\"Logic\":0}"
        };

        var result = _validator.Validate(resource);

        Assert.True(result.IsValid);
    }
}
