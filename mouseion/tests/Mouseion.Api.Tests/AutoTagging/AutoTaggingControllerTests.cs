// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.AutoTagging;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Tags.AutoTagging;

namespace Mouseion.Api.Tests.AutoTagging;

public class AutoTaggingControllerTests : ControllerTestBase
{
    public AutoTaggingControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetAllRules_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/autotagging/rules");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<List<AutoTaggingRuleResource>>();
        Assert.NotNull(result);
    }

    [Fact]
    public async Task GetRuleById_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/autotagging/rules/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task CreateRule_WithValidData_ReturnsCreated()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Test Rule",
            Enabled = true,
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(created);
        Assert.Equal("Test Rule", created.Name);
        Assert.True(created.Id > 0);
    }

    [Fact]
    public async Task CreateRule_WithEmptyName_ReturnsBadRequest()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task CreateRule_WithEmptyConditionValue_ReturnsBadRequest()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Test Rule",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "",
            TagId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task CreateRule_WithInvalidTagId_ReturnsBadRequest()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Test Rule",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 0
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task UpdateRule_WithExistingRule_ReturnsUpdated()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Original Rule",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        var created = await createResponse.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(created);

        created.Name = "Updated Rule";
        created.ConditionValue = "Comedy";

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/autotagging/rules/{created.Id}", created);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(updated);
        Assert.Equal("Updated Rule", updated.Name);
        Assert.Equal("Comedy", updated.ConditionValue);
    }

    [Fact]
    public async Task UpdateRule_WithNonExistentId_ReturnsNotFound()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Test Rule",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 1
        };

        var response = await Client.PutAsJsonAsync("/api/v3/autotagging/rules/99999", rule);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task DeleteRule_WithExistingRule_ReturnsNoContent()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Rule to Delete",
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Horror",
            TagId = 1
        };

        var createResponse = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        var created = await createResponse.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(created);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/autotagging/rules/{created.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/autotagging/rules/{created.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }

    [Fact]
    public async Task DeleteRule_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.DeleteAsync("/api/v3/autotagging/rules/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task ApplyRulesToAll_ReturnsResult()
    {
        var response = await Client.PostAsync("/api/v3/autotagging/apply", null);
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<ApplyAutoTagsResult>();
        Assert.NotNull(result);
    }

    [Fact]
    public async Task ApplyRulesToAll_WithMediaTypeFilter_ReturnsResult()
    {
        var response = await Client.PostAsync("/api/v3/autotagging/apply?mediaType=Movie", null);
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<ApplyAutoTagsResult>();
        Assert.NotNull(result);
    }

    [Fact]
    public async Task PreviewTags_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/autotagging/preview/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task CreateRule_WithMediaTypeFilter_ReturnsCreated()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Movie Only Rule",
            Enabled = true,
            ConditionType = AutoTaggingConditionType.GenreContains,
            ConditionValue = "Action",
            TagId = 1,
            MediaTypeFilter = MediaType.Movie
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(created);
        Assert.Equal(MediaType.Movie, created.MediaTypeFilter);
    }

    [Fact]
    public async Task CreateRule_WithDisabled_ReturnsCreated()
    {
        var rule = new AutoTaggingRuleResource
        {
            Name = "Disabled Rule",
            Enabled = false,
            ConditionType = AutoTaggingConditionType.LanguageContains,
            ConditionValue = "eng",
            TagId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/autotagging/rules", rule);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AutoTaggingRuleResource>();
        Assert.NotNull(created);
        Assert.False(created.Enabled);
    }
}
