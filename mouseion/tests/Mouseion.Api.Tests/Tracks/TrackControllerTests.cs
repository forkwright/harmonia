// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Albums;
using Mouseion.Api.Artists;
using Mouseion.Api.Common;
using Mouseion.Api.Tracks;

namespace Mouseion.Api.Tests.Tracks;

public class TrackControllerTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly HttpClient _client;

    public TrackControllerTests(TestWebApplicationFactory factory)
    {
        _client = factory.CreateClient();
        _client.DefaultRequestHeaders.Add("X-Api-Key", "test-api-key");
    }

    [Fact]
    public async Task GetTracks_ReturnsSuccessfully()
    {
        var response = await _client.GetAsync("/api/v3/tracks");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<TrackResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddTrack_ReturnsCreated_WithValidTrack()
    {
        var artist = new ArtistResource
        {
            Name = "The Doors",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await _client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "L.A. Woman",
            Monitored = true,
            QualityProfileId = 1
        };
        var albumResponse = await _client.PostAsJsonAsync("/api/v3/albums", album);
        var createdAlbum = await albumResponse.Content.ReadFromJsonAsync<AlbumResource>();

        var track = new TrackResource
        {
            AlbumId = createdAlbum!.Id,
            ArtistId = createdArtist.Id,
            Title = "Riders on the Storm",
            TrackNumber = 10,
            DiscNumber = 1,
            DurationSeconds = 432,
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await _client.PostAsJsonAsync("/api/v3/tracks", track);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("Riders on the Storm", created.Title);
        Assert.Equal(10, created.TrackNumber);
    }

    [Fact]
    public async Task GetTrack_ReturnsTrack_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "The Rolling Stones",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await _client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "Sticky Fingers",
            Monitored = true,
            QualityProfileId = 1
        };
        var albumResponse = await _client.PostAsJsonAsync("/api/v3/albums", album);
        var createdAlbum = await albumResponse.Content.ReadFromJsonAsync<AlbumResource>();

        var track = new TrackResource
        {
            AlbumId = createdAlbum!.Id,
            Title = "Brown Sugar",
            TrackNumber = 1,
            DiscNumber = 1,
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/tracks", track);
        var created = await createResponse.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(created);

        var getResponse = await _client.GetAsync($"/api/v3/tracks/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Brown Sugar", retrieved.Title);
    }

    [Fact]
    public async Task GetTrack_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await _client.GetAsync("/api/v3/tracks/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateTrack_UpdatesFields_WhenValid()
    {
        var artist = new ArtistResource
        {
            Name = "AC/DC",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await _client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "Back in Black",
            Monitored = true,
            QualityProfileId = 1
        };
        var albumResponse = await _client.PostAsJsonAsync("/api/v3/albums", album);
        var createdAlbum = await albumResponse.Content.ReadFromJsonAsync<AlbumResource>();

        var track = new TrackResource
        {
            AlbumId = createdAlbum!.Id,
            Title = "Hells Bells",
            TrackNumber = 1,
            DiscNumber = 1,
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/tracks", track);
        var created = await createResponse.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.DurationSeconds = 312;

        var updateResponse = await _client.PutAsJsonAsync($"/api/v3/tracks/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal(312, updated.DurationSeconds);
    }

    [Fact]
    public async Task DeleteTrack_ReturnsNoContent_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "Jimi Hendrix",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await _client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "Are You Experienced",
            Monitored = true,
            QualityProfileId = 1
        };
        var albumResponse = await _client.PostAsJsonAsync("/api/v3/albums", album);
        var createdAlbum = await albumResponse.Content.ReadFromJsonAsync<AlbumResource>();

        var track = new TrackResource
        {
            AlbumId = createdAlbum!.Id,
            Title = "Purple Haze",
            TrackNumber = 1,
            DiscNumber = 1,
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/tracks", track);
        var created = await createResponse.Content.ReadFromJsonAsync<TrackResource>();
        Assert.NotNull(created);

        var deleteResponse = await _client.DeleteAsync($"/api/v3/tracks/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await _client.GetAsync($"/api/v3/tracks/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
