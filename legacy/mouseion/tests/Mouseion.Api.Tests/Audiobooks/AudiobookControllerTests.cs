// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Audiobooks;
using Mouseion.Api.Authors;
using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Audiobooks;

public class AudiobookControllerTests : ControllerTestBase
{
    public AudiobookControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetAudiobooks_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/audiobooks");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<AudiobookResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddAudiobook_ReturnsCreated_WithValidAudiobook()
    {
        var author = await CreateAuthor("Brandon Sanderson");

        var audiobook = new AudiobookResource
        {
            Title = "The Way of Kings",
            Year = 2010,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id,
            Metadata = new AudiobookMetadataResource
            {
                Description = "Epic fantasy novel",
                Asin = "B003ZWFO7E",
                Narrator = "Michael Kramer, Kate Reading",
                DurationMinutes = 2745,
                IsAbridged = false,
                Publisher = "Macmillan Audio"
            }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/audiobooks", audiobook);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<AudiobookResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("The Way of Kings", created.Title);
        Assert.Equal("Michael Kramer, Kate Reading", created.Metadata.Narrator);
        Assert.Equal(2745, created.Metadata.DurationMinutes);
        Assert.False(created.Metadata.IsAbridged);
    }

    [Fact]
    public async Task GetAudiobooksByAuthor_ReturnsAudiobooks_ForExistingAuthor()
    {
        var author = await CreateAuthor("Patrick Rothfuss");

        var ab1 = await CreateAudiobook("The Name of the Wind", 2007, author.Id, "Nick Podehl");
        var ab2 = await CreateAudiobook("The Wise Man's Fear", 2011, author.Id, "Nick Podehl");

        var response = await Client.GetAsync($"/api/v3/audiobooks/author/{author.Id}");
        response.EnsureSuccessStatusCode();

        var audiobooks = await response.Content.ReadFromJsonAsync<List<AudiobookResource>>();
        Assert.NotNull(audiobooks);
        Assert.Equal(2, audiobooks.Count);
        Assert.All(audiobooks, ab => Assert.Equal("Nick Podehl", ab.Metadata.Narrator));
    }

    [Fact]
    public async Task GetStatistics_ReturnsNarratorData()
    {
        var author = await CreateAuthor("Brandon Sanderson");
        await CreateAudiobook("Mistborn: The Final Empire", 2006, author.Id, "Michael Kramer");
        await CreateAudiobook("Elantris", 2005, author.Id, "Jack Garrett");

        var response = await Client.GetAsync("/api/v3/audiobooks/statistics");
        response.EnsureSuccessStatusCode();

        var stats = await response.Content.ReadFromJsonAsync<dynamic>();
        Assert.NotNull(stats);
    }

    [Fact]
    public async Task GetNarratorStatistics_ReturnsNarratorSpecificData()
    {
        var author = await CreateAuthor("Jim Butcher");
        await CreateAudiobook("Storm Front", 2000, author.Id, "James Marsters");
        await CreateAudiobook("Fool Moon", 2001, author.Id, "James Marsters");

        var response = await Client.GetAsync("/api/v3/audiobooks/statistics/narrator/James%20Marsters");
        response.EnsureSuccessStatusCode();

        var stats = await response.Content.ReadFromJsonAsync<dynamic>();
        Assert.NotNull(stats);
    }

    [Fact]
    public async Task UpdateAudiobook_UpdatesNarratorField_WhenValid()
    {
        var author = await CreateAuthor("Joe Abercrombie");
        var audiobook = await CreateAudiobook("The Blade Itself", 2006, author.Id, "Steven Pacey");

        audiobook.Metadata.Description = "Grimdark fantasy narrated brilliantly";
        audiobook.Monitored = false;

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/audiobooks/{audiobook.Id}", audiobook);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<AudiobookResource>();
        Assert.NotNull(updated);
        Assert.False(updated.Monitored);
        Assert.Equal("Steven Pacey", updated.Metadata.Narrator);
        Assert.Contains("Grimdark", updated.Metadata.Description);
    }

    [Fact]
    public async Task DeleteAudiobook_ReturnsNoContent_WhenExists()
    {
        var author = await CreateAuthor("N.K. Jemisin");
        var audiobook = await CreateAudiobook("The Fifth Season", 2015, author.Id, "Robin Miles");

        var deleteResponse = await Client.DeleteAsync($"/api/v3/audiobooks/{audiobook.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/audiobooks/{audiobook.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }

    private async Task<AuthorResource> CreateAuthor(string name)
    {
        var author = new AuthorResource
        {
            Name = name,
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/authors", author);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);
        return created;
    }

    private async Task<AudiobookResource> CreateAudiobook(string title, int year, int authorId, string narrator)
    {
        var audiobook = new AudiobookResource
        {
            Title = title,
            Year = year,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = authorId,
            Metadata = new AudiobookMetadataResource
            {
                Narrator = narrator,
                DurationMinutes = 600,
                IsAbridged = false
            }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/audiobooks", audiobook);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AudiobookResource>();
        Assert.NotNull(created);
        return created;
    }
}
