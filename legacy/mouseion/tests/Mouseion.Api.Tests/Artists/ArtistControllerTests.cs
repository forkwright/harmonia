// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Artists;
using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Artists;

public class ArtistControllerTests : ControllerTestBase
{
    public ArtistControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }


    [Fact]
    public async Task GetArtists_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/artists/music");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<ArtistResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddArtist_ReturnsCreated_WithValidArtist()
    {
        var artist = new ArtistResource
        {
            Name = "The Beatles",
            SortName = "Beatles, The",
            Description = "Legendary rock band",
            MusicBrainzId = "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("The Beatles", created.Name);
        Assert.Equal("Beatles, The", created.SortName);
    }

    [Fact]
    public async Task GetArtist_ReturnsArtist_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "Pink Floyd",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var created = await createResponse.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(created);

        var getResponse = await Client.GetAsync($"/api/v3/artists/music/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Pink Floyd", retrieved.Name);
    }

    [Fact]
    public async Task GetArtist_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await Client.GetAsync("/api/v3/artists/music/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateArtist_UpdatesFields_WhenValid()
    {
        var artist = new ArtistResource
        {
            Name = "Led Zeppelin",
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var created = await createResponse.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.Description = "Hard rock pioneers";

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/artists/music/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal("Hard rock pioneers", updated.Description);
    }

    [Fact]
    public async Task DeleteArtist_ReturnsNoContent_WhenExists()
    {
        var artist = new ArtistResource
        {
            Name = "Queen",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/artists/music", artist);
        var created = await createResponse.Content.ReadFromJsonAsync<ArtistResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/artists/music/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/artists/music/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
