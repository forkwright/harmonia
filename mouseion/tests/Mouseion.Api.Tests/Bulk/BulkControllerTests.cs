// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Core.Bulk;

namespace Mouseion.Api.Tests.Bulk;

public class BulkControllerTests : ControllerTestBase
{
    public BulkControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task UpdateMovies_WithEmptyItems_ReturnsSuccess()
    {
        var request = new BulkUpdateRequest { Items = new List<BulkUpdateItem>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/movies/update", request);
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task DeleteMovies_WithEmptyIds_ReturnsSuccess()
    {
        var request = new BulkDeleteRequest { Ids = new List<int>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/movies/delete", request);
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task UpdateBooks_WithEmptyItems_ReturnsSuccess()
    {
        var request = new BulkUpdateRequest { Items = new List<BulkUpdateItem>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/books/update", request);
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task DeleteBooks_WithEmptyIds_ReturnsSuccess()
    {
        var request = new BulkDeleteRequest { Ids = new List<int>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/books/delete", request);
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task UpdateAudiobooks_WithEmptyItems_ReturnsSuccess()
    {
        var request = new BulkUpdateRequest { Items = new List<BulkUpdateItem>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/audiobooks/update", request);
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task DeleteAudiobooks_WithEmptyIds_ReturnsSuccess()
    {
        var request = new BulkDeleteRequest { Ids = new List<int>() };
        var response = await Client.PostAsJsonAsync("/api/v3/bulk/audiobooks/delete", request);
        response.EnsureSuccessStatusCode();
    }
}
