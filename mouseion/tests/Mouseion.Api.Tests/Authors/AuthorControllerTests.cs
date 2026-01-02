// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Authors;
using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Authors;

public class AuthorControllerTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly HttpClient _client;

    public AuthorControllerTests(TestWebApplicationFactory factory)
    {
        _client = factory.CreateClient();
        _client.DefaultRequestHeaders.Add("X-Api-Key", "test-api-key");
    }

    [Fact]
    public async Task GetAuthors_ReturnsSuccessfully()
    {
        var response = await _client.GetAsync("/api/v3/authors");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<AuthorResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddAuthor_ReturnsCreated_WithValidAuthor()
    {
        var author = new AuthorResource
        {
            Name = "Brandon Sanderson",
            SortName = "Sanderson, Brandon",
            Description = "Fantasy author",
            ForeignAuthorId = "ol123",
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await _client.PostAsJsonAsync("/api/v3/authors", author);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("Brandon Sanderson", created.Name);
        Assert.Equal("Sanderson, Brandon", created.SortName);
    }

    [Fact]
    public async Task GetAuthor_ReturnsAuthor_WhenExists()
    {
        var author = new AuthorResource
        {
            Name = "Patrick Rothfuss",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/authors", author);
        var created = await createResponse.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);

        var getResponse = await _client.GetAsync($"/api/v3/authors/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Patrick Rothfuss", retrieved.Name);
    }

    [Fact]
    public async Task GetAuthor_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await _client.GetAsync("/api/v3/authors/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateAuthor_UpdatesFields_WhenValid()
    {
        var author = new AuthorResource
        {
            Name = "Joe Abercrombie",
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/authors", author);
        var created = await createResponse.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.Description = "Grimdark fantasy author";

        var updateResponse = await _client.PutAsJsonAsync($"/api/v3/authors/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal("Grimdark fantasy author", updated.Description);
    }

    [Fact]
    public async Task DeleteAuthor_ReturnsNoContent_WhenExists()
    {
        var author = new AuthorResource
        {
            Name = "N.K. Jemisin",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await _client.PostAsJsonAsync("/api/v3/authors", author);
        var created = await createResponse.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);

        var deleteResponse = await _client.DeleteAsync($"/api/v3/authors/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await _client.GetAsync($"/api/v3/authors/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
