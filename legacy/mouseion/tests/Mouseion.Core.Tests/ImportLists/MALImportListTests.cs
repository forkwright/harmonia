// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Moq;
using Moq.Protected;
using Mouseion.Core.ImportLists;
using Mouseion.Core.ImportLists.MAL;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tests.ImportLists;

public class MALImportListTests
{
    private readonly Mock<ILogger<MALImportList>> _loggerMock;

    public MALImportListTests()
    {
        _loggerMock = new Mock<ILogger<MALImportList>>();
    }

    private MALImportList CreateImportList(HttpClient httpClient, MALSettings? settings = null)
    {
        var httpFactory = new Mock<IHttpClientFactory>();
        httpFactory.Setup(f => f.CreateClient(It.IsAny<string>())).Returns(httpClient);

        var importList = new MALImportList(_loggerMock.Object, httpFactory.Object);

        settings ??= new MALSettings
        {
            ClientId = "test-client-id",
            AccessToken = "test-access-token",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30)
        };

        importList.Definition = new ImportListDefinition
        {
            Id = 1,
            Settings = JsonSerializer.Serialize(settings)
        };

        return importList;
    }

    [Fact]
    public void Name_IsMAL()
    {
        var client = new HttpClient();
        var importList = CreateImportList(client);
        Assert.Equal("MyAnimeList", importList.Name);
    }

    [Fact]
    public void ListType_IsMAL()
    {
        var client = new HttpClient();
        var importList = CreateImportList(client);
        Assert.Equal(ImportListType.MAL, importList.ListType);
    }

    [Fact]
    public async Task Fetch_WithExpiredToken_ReturnsFailure()
    {
        var client = new HttpClient();
        var settings = new MALSettings
        {
            ClientId = "test",
            AccessToken = "expired",
            TokenExpiresAt = DateTime.UtcNow.AddDays(-1) // Expired
        };

        var importList = CreateImportList(client, settings);
        var result = await importList.FetchAsync();

        Assert.True(result.AnyFailure);
        Assert.Empty(result.Items);
    }

    [Fact]
    public async Task Fetch_WithNoToken_ReturnsFailure()
    {
        var client = new HttpClient();
        var settings = new MALSettings { ClientId = "test" };

        var importList = CreateImportList(client, settings);
        var result = await importList.FetchAsync();

        Assert.True(result.AnyFailure);
    }

    [Fact]
    public async Task Fetch_AnimeList_DeserializesCorrectly()
    {
        var animeResponse = new MALPagedResponse<MALAnimeListItem>
        {
            Data = new List<MALAnimeListItem>
            {
                new()
                {
                    Node = new MALAnimeNode { Id = 1, Title = "Naruto", StartDate = "2002-10-03" },
                    ListStatus = new MALAnimeListStatus { Status = "completed", Score = 8, FinishDate = "2007-02-08" }
                },
                new()
                {
                    Node = new MALAnimeNode { Id = 2, Title = "One Piece", StartDate = "1999-10-20" },
                    ListStatus = new MALAnimeListStatus { Status = "watching", Score = 9 }
                }
            }
        };

        var handler = CreateMockHandler(animeResponse);
        var httpClient = new HttpClient(handler);

        var settings = new MALSettings
        {
            ClientId = "test",
            AccessToken = "valid-token",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30),
            ImportAnimeList = true,
            ImportMangaList = false
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.False(result.AnyFailure);
        Assert.Equal(2, result.Items.Count);
        Assert.Equal("Naruto", result.Items[0].Title);
        Assert.Equal(1, result.Items[0].MalId);
        Assert.Equal(8, result.Items[0].UserRating);
        Assert.Equal("MAL", result.Items[0].ImportSource);
    }

    [Fact]
    public async Task Fetch_StatusFilter_ExcludesNonMatching()
    {
        var animeResponse = new MALPagedResponse<MALAnimeListItem>
        {
            Data = new List<MALAnimeListItem>
            {
                new()
                {
                    Node = new MALAnimeNode { Id = 1, Title = "Completed Anime" },
                    ListStatus = new MALAnimeListStatus { Status = "completed" }
                },
                new()
                {
                    Node = new MALAnimeNode { Id = 2, Title = "Plan to Watch" },
                    ListStatus = new MALAnimeListStatus { Status = "plan_to_watch" }
                }
            }
        };

        var handler = CreateMockHandler(animeResponse);
        var httpClient = new HttpClient(handler);

        var settings = new MALSettings
        {
            ClientId = "test",
            AccessToken = "valid",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30),
            ImportAnimeList = true,
            ImportMangaList = false,
            StatusFilter = new List<string> { "completed" }
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.Single(result.Items);
        Assert.Equal("Completed Anime", result.Items[0].Title);
    }

    [Fact]
    public void Settings_HasValidToken_WithExpiredDate_ReturnsFalse()
    {
        var settings = new MALSettings
        {
            AccessToken = "token",
            TokenExpiresAt = DateTime.UtcNow.AddMinutes(-5)
        };

        Assert.False(settings.HasValidToken);
    }

    [Fact]
    public void Settings_HasValidToken_WithFutureDate_ReturnsTrue()
    {
        var settings = new MALSettings
        {
            AccessToken = "token",
            TokenExpiresAt = DateTime.UtcNow.AddDays(30)
        };

        Assert.True(settings.HasValidToken);
    }

    private static HttpMessageHandler CreateMockHandler<T>(T responseBody)
    {
        var handler = new Mock<HttpMessageHandler>();
        handler.Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .ReturnsAsync(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent(JsonSerializer.Serialize(responseBody))
            });
        return handler.Object;
    }
}
