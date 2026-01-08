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

public class MovieControllerTests : ControllerTestBase
{
    public MovieControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetMovies_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/movies");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<MovieResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddMovie_ReturnsCreated_WithValidMovie()
    {
        var movie = new MovieResource
        {
            Title = "The Matrix",
            Year = 1999,
            TmdbId = "603",
            ImdbId = "tt0133093",
            Overview = "A computer hacker learns about the true nature of reality.",
            Runtime = 136,
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/movies", movie);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("The Matrix", created.Title);
        Assert.Equal(1999, created.Year);
    }

    [Fact]
    public async Task GetMovie_ReturnsMovie_WhenExists()
    {
        var movie = new MovieResource
        {
            Title = "Inception",
            Year = 2010,
            TmdbId = "27205",
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/movies", movie);
        var created = await createResponse.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(created);

        var getResponse = await Client.GetAsync($"/api/v3/movies/{created.Id}");
        getResponse.EnsureSuccessStatusCode();

        var retrieved = await getResponse.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(retrieved);
        Assert.Equal(created.Id, retrieved.Id);
        Assert.Equal("Inception", retrieved.Title);
    }

    [Fact]
    public async Task GetMovie_ReturnsNotFound_WhenDoesNotExist()
    {
        var response = await Client.GetAsync("/api/v3/movies/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateMovie_UpdatesFields_WhenValid()
    {
        var movie = new MovieResource
        {
            Title = "Interstellar",
            Year = 2014,
            Monitored = false,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/movies", movie);
        var created = await createResponse.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(created);

        created.Monitored = true;
        created.Runtime = 169;

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/movies/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(updated);
        Assert.True(updated.Monitored);
        Assert.Equal(169, updated.Runtime);
    }

    [Fact]
    public async Task DeleteMovie_ReturnsNoContent_WhenExists()
    {
        var movie = new MovieResource
        {
            Title = "The Dark Knight",
            Year = 2008,
            Monitored = true,
            QualityProfileId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/movies", movie);
        var created = await createResponse.Content.ReadFromJsonAsync<MovieResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/movies/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/movies/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }
}
