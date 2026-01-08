// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Moq;
using Mouseion.Core.Filtering;
using Mouseion.Core.Music;

namespace Mouseion.Core.Tests.Repositories;

public class TrackRepositoryTests : RepositoryTestBase
{
    private readonly ITrackRepository _repository;

    public TrackRepositoryTests()
    {
        var filterQueryBuilder = new FilterQueryBuilder();
        _repository = new TrackRepository(Database, filterQueryBuilder);
    }

    [Fact]
    public async Task AllAsync_ReturnsOnlyTracks()
    {
        var track1 = CreateTrack("Track 1");
        var track2 = CreateTrack("Track 2");
        await _repository.InsertAsync(track1);
        await _repository.InsertAsync(track2);

        var result = await _repository.AllAsync();

        Assert.Equal(2, result.Count());
        Assert.All(result, t => Assert.NotNull(t.Title));
    }

    [Fact]
    public void All_ReturnsOnlyTracks()
    {
        var track1 = CreateTrack("Track 1");
        var track2 = CreateTrack("Track 2");
        _repository.Insert(track1);
        _repository.Insert(track2);

        var result = _repository.All();

        Assert.Equal(2, result.Count());
        Assert.All(result, t => Assert.NotNull(t.Title));
    }

    [Fact]
    public async Task GetPageAsync_ReturnsPaginatedResults()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _repository.InsertAsync(CreateTrack($"Track {i}"));
        }

        var page1 = await _repository.GetPageAsync(1, 5);
        var page2 = await _repository.GetPageAsync(2, 5);

        Assert.Equal(5, page1.Count());
        Assert.Equal(5, page2.Count());
        Assert.NotEqual(page1.First().Id, page2.First().Id);
    }

    [Fact]
    public async Task FindAsync_ReturnsTrackById()
    {
        var track = CreateTrack("Findable Track");
        var inserted = await _repository.InsertAsync(track);

        var found = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Track", found.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsNullWhenNotFound()
    {
        var found = await _repository.FindAsync(999);

        Assert.Null(found);
    }

    [Fact]
    public void Find_ReturnsTrackById()
    {
        var track = CreateTrack("Findable Track");
        var inserted = _repository.Insert(track);

        var found = _repository.Find(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Track", found.Title);
    }

    [Fact]
    public async Task FindByForeignIdAsync_ReturnsMatchingTrack()
    {
        var track = CreateTrack("Foreign Track", foreignTrackId: "foreign-123");
        await _repository.InsertAsync(track);

        var found = await _repository.FindByForeignIdAsync("foreign-123");

        Assert.NotNull(found);
        Assert.Equal("foreign-123", found.ForeignTrackId);
        Assert.Equal("Foreign Track", found.Title);
    }

    [Fact]
    public async Task FindByForeignIdAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByForeignIdAsync("nonexistent-foreign-id");

        Assert.Null(found);
    }

    [Fact]
    public void FindByForeignId_ReturnsMatchingTrack()
    {
        var track = CreateTrack("Foreign Track", foreignTrackId: "foreign-123");
        _repository.Insert(track);

        var found = _repository.FindByForeignId("foreign-123");

        Assert.NotNull(found);
        Assert.Equal("foreign-123", found.ForeignTrackId);
    }

    [Fact]
    public async Task GetByAlbumIdAsync_ReturnsTracksOrderedByDiscAndTrackNumber()
    {
        var track1 = CreateTrack("Track 1", albumId: 1, trackNumber: 1, discNumber: 1);
        var track2 = CreateTrack("Track 2", albumId: 1, trackNumber: 2, discNumber: 1);
        var track3 = CreateTrack("Track 3", albumId: 1, trackNumber: 1, discNumber: 2);
        var track4 = CreateTrack("Track 4", albumId: 2, trackNumber: 1, discNumber: 1);

        await _repository.InsertAsync(track1);
        await _repository.InsertAsync(track2);
        await _repository.InsertAsync(track3);
        await _repository.InsertAsync(track4);

        var album1Tracks = await _repository.GetByAlbumIdAsync(1);

        Assert.Equal(3, album1Tracks.Count);
        Assert.All(album1Tracks, t => Assert.Equal(1, t.AlbumId));
        Assert.Equal(1, album1Tracks[0].DiscNumber);
        Assert.Equal(1, album1Tracks[0].TrackNumber);
        Assert.Equal(2, album1Tracks[1].TrackNumber);
    }

    [Fact]
    public void GetByAlbumId_ReturnsTracksForAlbum()
    {
        var track1 = CreateTrack("Track 1", albumId: 1);
        var track2 = CreateTrack("Track 2", albumId: 1);

        _repository.Insert(track1);
        _repository.Insert(track2);

        var album1Tracks = _repository.GetByAlbumId(1);

        Assert.Equal(2, album1Tracks.Count);
    }

    [Fact]
    public async Task GetByArtistIdAsync_ReturnsTracksByArtist()
    {
        var track1 = CreateTrack("Artist 1 Track 1", artistId: 1);
        var track2 = CreateTrack("Artist 1 Track 2", artistId: 1);
        var track3 = CreateTrack("Artist 2 Track 1", artistId: 2);

        await _repository.InsertAsync(track1);
        await _repository.InsertAsync(track2);
        await _repository.InsertAsync(track3);

        var artist1Tracks = await _repository.GetByArtistIdAsync(1);

        Assert.Equal(2, artist1Tracks.Count);
        Assert.All(artist1Tracks, t => Assert.Equal(1, t.ArtistId));
    }

    [Fact]
    public void GetByArtistId_ReturnsTracksByArtist()
    {
        var track1 = CreateTrack("Artist 1 Track 1", artistId: 1);
        var track2 = CreateTrack("Artist 1 Track 2", artistId: 1);

        _repository.Insert(track1);
        _repository.Insert(track2);

        var artist1Tracks = _repository.GetByArtistId(1);

        Assert.Equal(2, artist1Tracks.Count);
    }

    [Fact]
    public async Task GetByIdsAsync_ReturnsMultipleTracks()
    {
        var track1 = await _repository.InsertAsync(CreateTrack("Track 1"));
        var track2 = await _repository.InsertAsync(CreateTrack("Track 2"));
        var track3 = await _repository.InsertAsync(CreateTrack("Track 3"));

        var ids = new[] { track1.Id, track3.Id };
        var tracks = await _repository.GetByIdsAsync(ids);

        Assert.Equal(2, tracks.Count);
        Assert.Contains(tracks, t => t.Id == track1.Id);
        Assert.Contains(tracks, t => t.Id == track3.Id);
        Assert.DoesNotContain(tracks, t => t.Id == track2.Id);
    }

    [Fact]
    public async Task GetByIdsAsync_ReturnsEmptyListWhenNoIds()
    {
        var tracks = await _repository.GetByIdsAsync(Array.Empty<int>());

        Assert.Empty(tracks);
    }

    [Fact]
    public void GetByIds_ReturnsMultipleTracks()
    {
        var track1 = _repository.Insert(CreateTrack("Track 1"));
        var track2 = _repository.Insert(CreateTrack("Track 2"));

        var ids = new[] { track1.Id, track2.Id };
        var tracks = _repository.GetByIds(ids);

        Assert.Equal(2, tracks.Count);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredTracks()
    {
        var monitored1 = CreateTrack("Monitored 1", monitored: true);
        var monitored2 = CreateTrack("Monitored 2", monitored: true);
        var unmonitored = CreateTrack("Unmonitored", monitored: false);

        await _repository.InsertAsync(monitored1);
        await _repository.InsertAsync(monitored2);
        await _repository.InsertAsync(unmonitored);

        var monitoredTracks = await _repository.GetMonitoredAsync();

        Assert.Equal(2, monitoredTracks.Count);
        Assert.All(monitoredTracks, t => Assert.True(t.Monitored));
    }

    [Fact]
    public void GetMonitored_ReturnsOnlyMonitoredTracks()
    {
        var monitored1 = CreateTrack("Monitored 1", monitored: true);
        var unmonitored = CreateTrack("Unmonitored", monitored: false);

        _repository.Insert(monitored1);
        _repository.Insert(unmonitored);

        var monitoredTracks = _repository.GetMonitored();

        Assert.Single(monitoredTracks);
        Assert.True(monitoredTracks[0].Monitored);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesTrack()
    {
        var track = CreateTrack("Original Title");
        var inserted = await _repository.InsertAsync(track);

        inserted.Title = "Updated Title";
        inserted.TrackNumber = 99;
        await _repository.UpdateAsync(inserted);

        var updated = await _repository.FindAsync(inserted.Id);
        Assert.Equal("Updated Title", updated!.Title);
        Assert.Equal(99, updated.TrackNumber);
    }

    [Fact]
    public async Task DeleteAsync_RemovesTrack()
    {
        var track = CreateTrack("To Delete");
        var inserted = await _repository.InsertAsync(track);

        await _repository.DeleteAsync(inserted.Id);

        var deleted = await _repository.FindAsync(inserted.Id);
        Assert.Null(deleted);
    }

    [Fact]
    public async Task CountAsync_ReturnsCorrectCount()
    {
        await _repository.InsertAsync(CreateTrack("Track 1"));
        await _repository.InsertAsync(CreateTrack("Track 2"));

        var count = await _repository.CountAsync();

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task InsertManyAsync_InsertsMultipleTracks()
    {
        var tracks = new List<Track>
        {
            CreateTrack("Track 1"),
            CreateTrack("Track 2"),
            CreateTrack("Track 3")
        };

        await _repository.InsertManyAsync(tracks);

        var allTracks = await _repository.AllAsync();
        Assert.Equal(3, allTracks.Count());
    }

    [Fact]
    public async Task UpdateManyAsync_UpdatesMultipleTracks()
    {
        var track1 = await _repository.InsertAsync(CreateTrack("Track 1"));
        var track2 = await _repository.InsertAsync(CreateTrack("Track 2"));

        track1.Title = "Updated Track 1";
        track2.Title = "Updated Track 2";

        await _repository.UpdateManyAsync(new List<Track> { track1, track2 });

        var updated1 = await _repository.FindAsync(track1.Id);
        var updated2 = await _repository.FindAsync(track2.Id);

        Assert.Equal("Updated Track 1", updated1!.Title);
        Assert.Equal("Updated Track 2", updated2!.Title);
    }
}
