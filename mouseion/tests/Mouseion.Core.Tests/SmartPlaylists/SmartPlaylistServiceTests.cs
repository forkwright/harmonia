// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Filtering;
using Mouseion.Core.Library;
using Mouseion.Core.Music;
using Mouseion.Core.SmartPlaylists;
using Xunit;

namespace Mouseion.Core.Tests.SmartPlaylists;

public class SmartPlaylistServiceTests
{
    private readonly Mock<ISmartPlaylistRepository> _repositoryMock;
    private readonly Mock<ILibraryFilterService> _filterServiceMock;
    private readonly Mock<ILogger<SmartPlaylistService>> _loggerMock;
    private readonly SmartPlaylistService _service;

    public SmartPlaylistServiceTests()
    {
        _repositoryMock = new Mock<ISmartPlaylistRepository>();
        _filterServiceMock = new Mock<ILibraryFilterService>();
        _loggerMock = new Mock<ILogger<SmartPlaylistService>>();
        _service = new SmartPlaylistService(_repositoryMock.Object, _filterServiceMock.Object, _loggerMock.Object);
    }

    [Fact]
    public async Task GetAllAsync_ReturnsAllPlaylists()
    {
        var playlists = new List<SmartPlaylist>
        {
            new() { Id = 1, Name = "Rock" },
            new() { Id = 2, Name = "Jazz" }
        };
        _repositoryMock.Setup(r => r.AllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlists);

        var result = await _service.GetAllAsync();

        Assert.Equal(2, result.Count());
        _repositoryMock.Verify(r => r.AllAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task GetAsync_WithValidId_ReturnsPlaylist()
    {
        var playlist = new SmartPlaylist { Id = 1, Name = "Rock" };
        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlist);

        var result = await _service.GetAsync(1);

        Assert.NotNull(result);
        Assert.Equal("Rock", result.Name);
    }

    [Fact]
    public async Task GetAsync_WithInvalidId_ReturnsNull()
    {
        _repositoryMock.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        var result = await _service.GetAsync(999);

        Assert.Null(result);
    }

    [Fact]
    public async Task CreateAsync_SetsTimestampsAndInserts()
    {
        var playlist = new SmartPlaylist { Name = "New Playlist" };
        var created = new SmartPlaylist { Id = 1, Name = "New Playlist" };
        _repositoryMock.Setup(r => r.InsertAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(created);

        var result = await _service.CreateAsync(playlist);

        Assert.Equal(1, result.Id);
        _repositoryMock.Verify(r => r.InsertAsync(
            It.Is<SmartPlaylist>(p =>
                p.CreatedAt != default &&
                p.UpdatedAt != default &&
                p.TrackCount == 0),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateAsync_SetsUpdatedAtAndUpdates()
    {
        var playlist = new SmartPlaylist { Id = 1, Name = "Updated" };
        _repositoryMock.Setup(r => r.UpdateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlist);

        var result = await _service.UpdateAsync(playlist);

        Assert.Equal("Updated", result.Name);
        _repositoryMock.Verify(r => r.UpdateAsync(
            It.Is<SmartPlaylist>(p => p.UpdatedAt != default),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteAsync_CallsRepositoryDelete()
    {
        await _service.DeleteAsync(1);

        _repositoryMock.Verify(r => r.DeleteAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RefreshAsync_WithValidId_RefreshesAndReturnsTracks()
    {
        var playlist = new SmartPlaylist
        {
            Id = 1,
            Name = "Rock",
            FilterRequestJson = "{\"Conditions\":[],\"Logic\":0,\"Page\":1,\"PageSize\":50}"
        };
        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlist);

        var tracks = new List<Track>
        {
            new() { Id = 10, Title = "Track A" },
            new() { Id = 20, Title = "Track B" },
            new() { Id = 30, Title = "Track C" }
        };
        _filterServiceMock.Setup(f => f.FilterTracksAsync(It.IsAny<FilterRequest>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new FilterResult { Tracks = tracks, TotalCount = 3 });

        _repositoryMock.Setup(r => r.UpdateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist p, CancellationToken _) => p);

        var result = await _service.RefreshAsync(1);

        Assert.Equal(3, result.TrackCount);
        _repositoryMock.Verify(r => r.SetTracksAsync(1,
            It.Is<IList<SmartPlaylistTrack>>(t => t.Count == 3 && t[0].TrackId == 10 && t[2].Position == 2),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RefreshAsync_WithInvalidId_ThrowsKeyNotFoundException()
    {
        _repositoryMock.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist?)null);

        await Assert.ThrowsAsync<KeyNotFoundException>(() => _service.RefreshAsync(999));
    }

    [Fact]
    public async Task GetTracksAsync_DelegatesToRepository()
    {
        var tracks = new List<SmartPlaylistTrack>
        {
            new() { SmartPlaylistId = 1, TrackId = 10, Position = 0 },
            new() { SmartPlaylistId = 1, TrackId = 20, Position = 1 }
        };
        _repositoryMock.Setup(r => r.GetTracksAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(tracks);

        var result = await _service.GetTracksAsync(1);

        Assert.Equal(2, result.Count());
    }

    [Fact]
    public async Task RefreshAsync_SetsPageSizeToMaxForFullScan()
    {
        var playlist = new SmartPlaylist
        {
            Id = 1,
            Name = "All",
            FilterRequestJson = "{\"Conditions\":[],\"Logic\":0,\"Page\":1,\"PageSize\":10}"
        };
        _repositoryMock.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(playlist);
        _filterServiceMock.Setup(f => f.FilterTracksAsync(
            It.Is<FilterRequest>(fr => fr.PageSize == int.MaxValue),
            It.IsAny<CancellationToken>()))
            .ReturnsAsync(new FilterResult { Tracks = new List<Track>(), TotalCount = 0 });
        _repositoryMock.Setup(r => r.UpdateAsync(It.IsAny<SmartPlaylist>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((SmartPlaylist p, CancellationToken _) => p);

        await _service.RefreshAsync(1);

        _filterServiceMock.Verify(f => f.FilterTracksAsync(
            It.Is<FilterRequest>(fr => fr.PageSize == int.MaxValue),
            It.IsAny<CancellationToken>()), Times.Once);
    }
}

public class SmartPlaylistEntityTests
{
    [Fact]
    public void SmartPlaylist_DefaultValues()
    {
        var playlist = new SmartPlaylist();

        Assert.Equal(0, playlist.Id);
        Assert.Equal(string.Empty, playlist.Name);
        Assert.Equal("{}", playlist.FilterRequestJson);
        Assert.Equal(0, playlist.TrackCount);
    }

    [Fact]
    public void SmartPlaylist_CanSetAllProperties()
    {
        var now = DateTime.UtcNow;
        var playlist = new SmartPlaylist
        {
            Id = 1,
            Name = "My Playlist",
            FilterRequestJson = "{\"Conditions\":[{\"Field\":\"Genre\",\"Operator\":0,\"Value\":\"Rock\"}]}",
            TrackCount = 42,
            LastRefreshed = now,
            CreatedAt = now,
            UpdatedAt = now
        };

        Assert.Equal(1, playlist.Id);
        Assert.Equal("My Playlist", playlist.Name);
        Assert.Contains("Rock", playlist.FilterRequestJson);
        Assert.Equal(42, playlist.TrackCount);
        Assert.Equal(now, playlist.LastRefreshed);
    }

    [Fact]
    public void SmartPlaylistTrack_DefaultValues()
    {
        var track = new SmartPlaylistTrack();

        Assert.Equal(0, track.SmartPlaylistId);
        Assert.Equal(0, track.TrackId);
        Assert.Equal(0, track.Position);
    }

    [Fact]
    public void SmartPlaylistTrack_CanSetAllProperties()
    {
        var track = new SmartPlaylistTrack
        {
            Id = 1,
            SmartPlaylistId = 5,
            TrackId = 100,
            Position = 3
        };

        Assert.Equal(1, track.Id);
        Assert.Equal(5, track.SmartPlaylistId);
        Assert.Equal(100, track.TrackId);
        Assert.Equal(3, track.Position);
    }
}
