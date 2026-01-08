// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Common;
using Mouseion.Api.Movies;

namespace Mouseion.Api.Tests.Movies;

public class CollectionControllerTests : ControllerTestBase
{
    public CollectionControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetCollections_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/collections");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<CollectionResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddCollection_ReturnsCreated_WithValidCollection()
    {
        var collection = new CollectionResource
        {
            Title = "The Lord of the Rings Collection",
            TmdbId = "119",
            Overview = "Epic fantasy trilogy",
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/collections", collection);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("The Lord of the Rings Collection", created.Title);
    }

    [Fact]
    public async Task GetCollection_ReturnsCollection_WhenExists()
    {
        var collection = new CollectionResource
        {
            Title = "Star Wars Collection",
            TmdbId = "10",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/collections", collection);
        var created = await createResponse.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(created);

        var getResponse = await Client.GetAsync($"/api/v3/collections/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Star Wars Collection", retrieved.Title);
    }

    [Fact]
    public async Task GetCollection_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await Client.GetAsync("/api/v3/collections/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateCollection_UpdatesFields_WhenValid()
    {
        var collection = new CollectionResource
        {
            Title = "Marvel Cinematic Universe",
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/collections", collection);
        var created = await createResponse.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.Overview = "Superhero film series";

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/collections/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal("Superhero film series", updated.Overview);
    }

    [Fact]
    public async Task DeleteCollection_ReturnsNoContent_WhenExists()
    {
        var collection = new CollectionResource
        {
            Title = "The Matrix Collection",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/collections", collection);
        var created = await createResponse.Content.ReadFromJsonAsync<CollectionResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/collections/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/collections/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
