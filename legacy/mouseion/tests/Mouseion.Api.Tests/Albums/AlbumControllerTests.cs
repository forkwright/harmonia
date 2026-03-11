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

namespace Mouseion.Api.Tests.Albums;

public class AlbumControllerTests : ControllerTestBase
{
    public AlbumControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }


    [Fact]
    public async Task GetAlbums_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/albums");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<AlbumResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddAlbum_ReturnsCreated_WithValidAlbum()
    {
        var artist = new ArtistResource
        {
            Name = "Radiohead",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "OK Computer",
            AlbumType = "Album",
            ReleaseDate = new DateTime(1997, 5, 21),
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/albums", album);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("OK Computer", created.Title);
        Assert.Equal(createdArtist.Id, created.ArtistId);
    }

    [Fact]
    public async Task GetAlbum_ReturnsAlbum_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "Nirvana",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "Nevermind",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/albums", album);
        var created = await createResponse.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(created);

        var getResponse = await Client.GetAsync($"/api/v3/albums/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Nevermind", retrieved.Title);
    }

    [Fact]
    public async Task GetAlbum_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await Client.GetAsync("/api/v3/albums/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateAlbum_UpdatesFields_WhenValid()
    {
        var artist = new ArtistResource
        {
            Name = "David Bowie",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "The Rise and Fall of Ziggy Stardust",
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/albums", album);
        var created = await createResponse.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.Description = "Concept album";

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/albums/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal("Concept album", updated.Description);
    }

    [Fact]
    public async Task DeleteAlbum_ReturnsNoContent_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "Metallica",
            Monitored = true,
            QualityProfileId = 1
        };
        var artistResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var createdArtist = await artistResponse.Content.ReadFromJsonAsync<ArtistResource>();

        var album = new AlbumResource
        {
            ArtistId = createdArtist!.Id,
            Title = "Master of Puppets",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/albums", album);
        var created = await createResponse.Content.ReadFromJsonAsync<AlbumResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/albums/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/albums/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
