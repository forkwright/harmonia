// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Tags;

namespace Mouseion.Api.Tests.Tags;

public class TagControllerTests : ControllerTestBase
{
    public TagControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetAll_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/tags");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<List<TagResource>>();
        Assert.NotNull(result);
    }

    [Fact]
    public async Task GetById_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/tags/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Create_WithValidData_ReturnsCreated()
    {
        var tag = new TagResource { Label = "test-tag" };

        var response = await Client.PostAsJsonAsync("/api/v3/tags", tag);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<TagResource>();
        Assert.NotNull(created);
        Assert.Equal("test-tag", created.Label);
        Assert.True(created.Id > 0);
    }

    [Fact]
    public async Task Create_WithInvalidLabel_ReturnsBadRequest()
    {
        var tag = new TagResource { Label = "Invalid Label With Spaces" };

        var response = await Client.PostAsJsonAsync("/api/v3/tags", tag);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_WithEmptyLabel_ReturnsBadRequest()
    {
        var tag = new TagResource { Label = "" };

        var response = await Client.PostAsJsonAsync("/api/v3/tags", tag);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Update_WithExistingTag_ReturnsUpdated()
    {
        var tag = new TagResource { Label = "original-tag" };
        var createResponse = await Client.PostAsJsonAsync("/api/v3/tags", tag);
        var created = await createResponse.Content.ReadFromJsonAsync<TagResource>();
        Assert.NotNull(created);

        created.Label = "updated-tag";
        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/tags/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<TagResource>();
        Assert.NotNull(updated);
        Assert.Equal("updated-tag", updated.Label);
    }

    [Fact]
    public async Task Update_WithNonExistentId_ReturnsNotFound()
    {
        var tag = new TagResource { Label = "test-tag" };

        var response = await Client.PutAsJsonAsync("/api/v3/tags/99999", tag);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Delete_WithExistingTag_ReturnsNoContent()
    {
        var tag = new TagResource { Label = "tag-to-delete" };
        var createResponse = await Client.PostAsJsonAsync("/api/v3/tags", tag);
        var created = await createResponse.Content.ReadFromJsonAsync<TagResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/tags/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/tags/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }

    [Fact]
    public async Task Delete_WithNonExistentId_ReturnsNoContent()
    {
        var response = await Client.DeleteAsync("/api/v3/tags/99999");
        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
    }

    [Fact]
    public async Task BulkApply_WithInvalidTagId_ReturnsBadRequest()
    {
        var request = new BulkTagRequest
        {
            TagId = 0,
            MediaItemIds = new List<int> { 1, 2, 3 }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/apply", request);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BulkApply_WithEmptyMediaItemIds_ReturnsBadRequest()
    {
        var request = new BulkTagRequest
        {
            TagId = 1,
            MediaItemIds = new List<int>()
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/apply", request);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BulkApply_WithValidRequest_ReturnsResult()
    {
        var request = new BulkTagRequest
        {
            TagId = 1,
            MediaItemIds = new List<int> { 99999 }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/apply", request);
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<BulkTagResult>();
        Assert.NotNull(result);
        Assert.Equal(0, result.UpdatedItems);
    }

    [Fact]
    public async Task BulkRemove_WithInvalidTagId_ReturnsBadRequest()
    {
        var request = new BulkTagRequest
        {
            TagId = 0,
            MediaItemIds = new List<int> { 1, 2, 3 }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/remove", request);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BulkRemove_WithEmptyMediaItemIds_ReturnsBadRequest()
    {
        var request = new BulkTagRequest
        {
            TagId = 1,
            MediaItemIds = new List<int>()
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/remove", request);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task BulkRemove_WithValidRequest_ReturnsResult()
    {
        var request = new BulkTagRequest
        {
            TagId = 1,
            MediaItemIds = new List<int> { 99999 }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/tags/bulk/remove", request);
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<BulkTagResult>();
        Assert.NotNull(result);
        Assert.Equal(0, result.UpdatedItems);
    }
}
