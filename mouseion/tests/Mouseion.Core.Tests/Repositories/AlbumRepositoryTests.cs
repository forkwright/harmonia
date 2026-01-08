// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Music;

namespace Mouseion.Core.Tests.Repositories;

public class AlbumRepositoryTests : RepositoryTestBase
{
    private readonly IAlbumRepository _repository;

    public AlbumRepositoryTests()
    {
        _repository = new AlbumRepository(Database);
    }

    [Fact]
    public async Task AllAsync_ReturnsOnlyAlbums()
    {
        var album1 = CreateAlbum("Album 1");
        var album2 = CreateAlbum("Album 2");
        await _repository.InsertAsync(album1);
        await _repository.InsertAsync(album2);

        var result = await _repository.AllAsync();

        Assert.Equal(2, result.Count());
        Assert.All(result, a => Assert.NotNull(a.Title));
    }

    [Fact]
    public void All_ReturnsOnlyAlbums()
    {
        var album1 = CreateAlbum("Album 1");
        var album2 = CreateAlbum("Album 2");
        _repository.Insert(album1);
        _repository.Insert(album2);

        var result = _repository.All();

        Assert.Equal(2, result.Count());
        Assert.All(result, a => Assert.NotNull(a.Title));
    }

    [Fact]
    public async Task GetPageAsync_ReturnsPaginatedResults()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _repository.InsertAsync(CreateAlbum($"Album {i}"));
        }

        var page1 = await _repository.GetPageAsync(1, 5);
        var page2 = await _repository.GetPageAsync(2, 5);

        Assert.Equal(5, page1.Count());
        Assert.Equal(5, page2.Count());
        Assert.NotEqual(page1.First().Id, page2.First().Id);
    }

    [Fact]
    public async Task FindByTitleAsync_WithArtistId_ReturnsMatchingAlbum()
    {
        var album = CreateAlbum("Unique Album", artistId: 1);
        await _repository.InsertAsync(album);

        var found = await _repository.FindByTitleAsync("Unique Album", 1);

        Assert.NotNull(found);
        Assert.Equal("Unique Album", found.Title);
        Assert.Equal(1, found.ArtistId);
    }

    [Fact]
    public async Task FindByTitleAsync_WithoutArtistId_ReturnsMatchingAlbum()
    {
        var album = CreateAlbum("Unique Album");
        await _repository.InsertAsync(album);

        var found = await _repository.FindByTitleAsync("Unique Album", null);

        Assert.NotNull(found);
        Assert.Equal("Unique Album", found.Title);
    }

    [Fact]
    public async Task FindByTitleAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByTitleAsync("Nonexistent Album", 1);

        Assert.Null(found);
    }

    [Fact]
    public void FindByTitle_WithArtistId_ReturnsMatchingAlbum()
    {
        var album = CreateAlbum("Unique Album", artistId: 1);
        _repository.Insert(album);

        var found = _repository.FindByTitle("Unique Album", 1);

        Assert.NotNull(found);
        Assert.Equal("Unique Album", found.Title);
    }

    [Fact]
    public async Task FindByForeignIdAsync_ReturnsMatchingAlbum()
    {
        var album = CreateAlbum("Foreign Album", foreignAlbumId: "foreign-123");
        await _repository.InsertAsync(album);

        var found = await _repository.FindByForeignIdAsync("foreign-123");

        Assert.NotNull(found);
        Assert.Equal("foreign-123", found.ForeignAlbumId);
        Assert.Equal("Foreign Album", found.Title);
    }

    [Fact]
    public async Task FindByForeignIdAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByForeignIdAsync("nonexistent-foreign-id");

        Assert.Null(found);
    }

    [Fact]
    public void FindByForeignId_ReturnsMatchingAlbum()
    {
        var album = CreateAlbum("Foreign Album", foreignAlbumId: "foreign-123");
        _repository.Insert(album);

        var found = _repository.FindByForeignId("foreign-123");

        Assert.NotNull(found);
        Assert.Equal("foreign-123", found.ForeignAlbumId);
    }

    [Fact]
    public async Task GetByArtistIdAsync_ReturnsAlbumsByArtist()
    {
        var album1 = CreateAlbum("Artist 1 Album 1", artistId: 1);
        var album2 = CreateAlbum("Artist 1 Album 2", artistId: 1);
        var album3 = CreateAlbum("Artist 2 Album 1", artistId: 2);

        await _repository.InsertAsync(album1);
        await _repository.InsertAsync(album2);
        await _repository.InsertAsync(album3);

        var artist1Albums = await _repository.GetByArtistIdAsync(1);

        Assert.Equal(2, artist1Albums.Count);
        Assert.All(artist1Albums, a => Assert.Equal(1, a.ArtistId));
    }

    [Fact]
    public void GetByArtistId_ReturnsAlbumsByArtist()
    {
        var album1 = CreateAlbum("Artist 1 Album 1", artistId: 1);
        var album2 = CreateAlbum("Artist 1 Album 2", artistId: 1);

        _repository.Insert(album1);
        _repository.Insert(album2);

        var artist1Albums = _repository.GetByArtistId(1);

        Assert.Equal(2, artist1Albums.Count);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredAlbums()
    {
        var monitored1 = CreateAlbum("Monitored 1", monitored: true);
        var monitored2 = CreateAlbum("Monitored 2", monitored: true);
        var unmonitored = CreateAlbum("Unmonitored", monitored: false);

        await _repository.InsertAsync(monitored1);
        await _repository.InsertAsync(monitored2);
        await _repository.InsertAsync(unmonitored);

        var monitoredAlbums = await _repository.GetMonitoredAsync();

        Assert.Equal(2, monitoredAlbums.Count);
        Assert.All(monitoredAlbums, a => Assert.True(a.Monitored));
    }

    [Fact]
    public void GetMonitored_ReturnsOnlyMonitoredAlbums()
    {
        var monitored1 = CreateAlbum("Monitored 1", monitored: true);
        var unmonitored = CreateAlbum("Unmonitored", monitored: false);

        _repository.Insert(monitored1);
        _repository.Insert(unmonitored);

        var monitoredAlbums = _repository.GetMonitored();

        Assert.Single(monitoredAlbums);
        Assert.True(monitoredAlbums[0].Monitored);
    }

    [Fact]
    public async Task AlbumExistsAsync_ReturnsTrueWhenExists()
    {
        var album = CreateAlbum("Existing Album", artistId: 1);
        await _repository.InsertAsync(album);

        var exists = await _repository.AlbumExistsAsync(1, "Existing Album");

        Assert.True(exists);
    }

    [Fact]
    public async Task AlbumExistsAsync_ReturnsFalseWhenNotExists()
    {
        var exists = await _repository.AlbumExistsAsync(1, "Nonexistent Album");

        Assert.False(exists);
    }

    [Fact]
    public void AlbumExists_ReturnsTrueWhenExists()
    {
        var album = CreateAlbum("Existing Album", artistId: 1);
        _repository.Insert(album);

        var exists = _repository.AlbumExists(1, "Existing Album");

        Assert.True(exists);
    }

    [Fact]
    public async Task GetVersionsAsync_ReturnsAlbumsWithSameReleaseGroup()
    {
        var version1 = CreateAlbum("Album Version 1", releaseGroupMbid: "rg-123");
        var version2 = CreateAlbum("Album Version 2", releaseGroupMbid: "rg-123");
        var different = CreateAlbum("Different Album", releaseGroupMbid: "rg-456");

        await _repository.InsertAsync(version1);
        await _repository.InsertAsync(version2);
        await _repository.InsertAsync(different);

        var versions = await _repository.GetVersionsAsync("rg-123");

        Assert.Equal(2, versions.Count);
        Assert.All(versions, a => Assert.Equal("rg-123", a.ReleaseGroupMbid));
    }

    [Fact]
    public void GetVersions_ReturnsAlbumsWithSameReleaseGroup()
    {
        var version1 = CreateAlbum("Album Version 1", releaseGroupMbid: "rg-123");
        var version2 = CreateAlbum("Album Version 2", releaseGroupMbid: "rg-123");

        _repository.Insert(version1);
        _repository.Insert(version2);

        var versions = _repository.GetVersions("rg-123");

        Assert.Equal(2, versions.Count);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesAlbum()
    {
        var album = CreateAlbum("Original Title");
        var inserted = await _repository.InsertAsync(album);

        inserted.Title = "Updated Title";
        inserted.ForeignAlbumId = "updated-foreign-id";
        await _repository.UpdateAsync(inserted);

        var updated = await _repository.FindAsync(inserted.Id);
        Assert.Equal("Updated Title", updated!.Title);
        Assert.Equal("updated-foreign-id", updated.ForeignAlbumId);
    }

    [Fact]
    public async Task DeleteAsync_RemovesAlbum()
    {
        var album = CreateAlbum("To Delete");
        var inserted = await _repository.InsertAsync(album);

        await _repository.DeleteAsync(inserted.Id);

        var deleted = await _repository.FindAsync(inserted.Id);
        Assert.Null(deleted);
    }

    [Fact]
    public async Task CountAsync_ReturnsCorrectCount()
    {
        await _repository.InsertAsync(CreateAlbum("Album 1"));
        await _repository.InsertAsync(CreateAlbum("Album 2"));

        var count = await _repository.CountAsync();

        Assert.Equal(2, count);
    }
}
