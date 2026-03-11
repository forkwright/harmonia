// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Moq;
using Moq.Protected;
using Mouseion.Core.ImportLists;
using Mouseion.Core.ImportLists.AniList;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tests.ImportLists;

public class AniListImportListTests
{
    private readonly Mock<ILogger<AniListImportList>> _loggerMock;

    public AniListImportListTests()
    {
        _loggerMock = new Mock<ILogger<AniListImportList>>();
    }

    private AniListImportList CreateImportList(HttpClient httpClient, AniListSettings? settings = null)
    {
        var httpFactory = new Mock<IHttpClientFactory>();
        httpFactory.Setup(f => f.CreateClient(It.IsAny<string>())).Returns(httpClient);

        var importList = new AniListImportList(_loggerMock.Object, httpFactory.Object);

        settings ??= new AniListSettings
        {
            Username = "testuser"
        };

        importList.Definition = new ImportListDefinition
        {
            Id = 1,
            Settings = JsonSerializer.Serialize(settings)
        };

        return importList;
    }

    [Fact]
    public void Name_IsAniList()
    {
        var client = new HttpClient();
        var importList = CreateImportList(client);
        Assert.Equal("AniList", importList.Name);
    }

    [Fact]
    public void ListType_IsAniList()
    {
        var client = new HttpClient();
        var importList = CreateImportList(client);
        Assert.Equal(ImportListType.AniList, importList.ListType);
    }

    [Fact]
    public async Task Fetch_WithNoCredentials_ReturnsFailure()
    {
        var client = new HttpClient();
        var settings = new AniListSettings { Username = "", AccessToken = "" };

        var importList = CreateImportList(client, settings);
        var result = await importList.FetchAsync();

        Assert.True(result.AnyFailure);
    }

    [Fact]
    public async Task Fetch_AnimeList_ParsesGraphQLResponse()
    {
        var graphqlResponse = new AniListGraphQLResponse<AniListMediaListCollectionData>
        {
            Data = new AniListMediaListCollectionData
            {
                MediaListCollection = new AniListMediaListCollection
                {
                    Lists = new List<AniListMediaListGroup>
                    {
                        new()
                        {
                            Name = "Completed",
                            Status = "COMPLETED",
                            Entries = new List<AniListMediaListEntry>
                            {
                                new()
                                {
                                    Id = 1,
                                    Status = "COMPLETED",
                                    Score = 9.0,
                                    Progress = 24,
                                    Media = new AniListMedia
                                    {
                                        Id = 101,
                                        IdMal = 201,
                                        Title = new AniListTitle
                                        {
                                            English = "Death Note",
                                            Romaji = "Death Note",
                                            Native = "デスノート"
                                        },
                                        Type = "ANIME",
                                        StartDate = new AniListFuzzyDate { Year = 2006, Month = 10, Day = 4 }
                                    },
                                    CompletedAt = new AniListFuzzyDate { Year = 2007, Month = 6, Day = 27 }
                                }
                            }
                        }
                    },
                    HasNextChunk = false
                }
            }
        };

        var handler = CreateMockHandler(graphqlResponse);
        var httpClient = new HttpClient(handler);

        var settings = new AniListSettings
        {
            Username = "testuser",
            ImportAnimeList = true,
            ImportMangaList = false
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.False(result.AnyFailure);
        Assert.Single(result.Items);

        var item = result.Items[0];
        Assert.Equal("Death Note", item.Title);
        Assert.Equal(101, item.AniListId);
        Assert.Equal(201, item.MalId);
        Assert.Equal(9, item.UserRating);
        Assert.Equal(2006, item.Year);
        Assert.Equal("AniList", item.ImportSource);
    }

    [Fact]
    public async Task Fetch_MangaList_SetsMangaMediaType()
    {
        var graphqlResponse = new AniListGraphQLResponse<AniListMediaListCollectionData>
        {
            Data = new AniListMediaListCollectionData
            {
                MediaListCollection = new AniListMediaListCollection
                {
                    Lists = new List<AniListMediaListGroup>
                    {
                        new()
                        {
                            Entries = new List<AniListMediaListEntry>
                            {
                                new()
                                {
                                    Status = "CURRENT",
                                    Media = new AniListMedia
                                    {
                                        Id = 301,
                                        Title = new AniListTitle { Romaji = "Berserk" },
                                        Type = "MANGA"
                                    }
                                }
                            }
                        }
                    },
                    HasNextChunk = false
                }
            }
        };

        var handler = CreateMockHandler(graphqlResponse);
        var httpClient = new HttpClient(handler);

        var settings = new AniListSettings
        {
            Username = "testuser",
            ImportAnimeList = false,
            ImportMangaList = true
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.Single(result.Items);
        Assert.Equal(MediaType.Manga, result.Items[0].MediaType);
        Assert.Equal("Berserk", result.Items[0].Title);
    }

    [Fact]
    public async Task Fetch_StatusFilter_FiltersEntries()
    {
        var graphqlResponse = new AniListGraphQLResponse<AniListMediaListCollectionData>
        {
            Data = new AniListMediaListCollectionData
            {
                MediaListCollection = new AniListMediaListCollection
                {
                    Lists = new List<AniListMediaListGroup>
                    {
                        new()
                        {
                            Entries = new List<AniListMediaListEntry>
                            {
                                new()
                                {
                                    Status = "COMPLETED",
                                    Media = new AniListMedia
                                    {
                                        Id = 1,
                                        Title = new AniListTitle { English = "Completed Show" },
                                        Type = "ANIME"
                                    }
                                },
                                new()
                                {
                                    Status = "PLANNING",
                                    Media = new AniListMedia
                                    {
                                        Id = 2,
                                        Title = new AniListTitle { English = "Planned Show" },
                                        Type = "ANIME"
                                    }
                                }
                            }
                        }
                    },
                    HasNextChunk = false
                }
            }
        };

        var handler = CreateMockHandler(graphqlResponse);
        var httpClient = new HttpClient(handler);

        var settings = new AniListSettings
        {
            Username = "testuser",
            ImportAnimeList = true,
            ImportMangaList = false,
            StatusFilter = new List<string> { "COMPLETED" }
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.Single(result.Items);
        Assert.Equal("Completed Show", result.Items[0].Title);
    }

    [Fact]
    public void AniListFuzzyDate_ToDateTime_ValidDate()
    {
        var date = new AniListFuzzyDate { Year = 2023, Month = 6, Day = 15 };
        var result = date.ToDateTime();

        Assert.NotNull(result);
        Assert.Equal(new DateTime(2023, 6, 15), result.Value);
    }

    [Fact]
    public void AniListFuzzyDate_ToDateTime_YearOnly()
    {
        var date = new AniListFuzzyDate { Year = 2023 };
        var result = date.ToDateTime();

        Assert.NotNull(result);
        Assert.Equal(new DateTime(2023, 1, 1), result.Value);
    }

    [Fact]
    public void AniListFuzzyDate_ToDateTime_NoYear_ReturnsNull()
    {
        var date = new AniListFuzzyDate();
        Assert.Null(date.ToDateTime());
    }

    [Fact]
    public void Settings_HasValidCredentials_WithUsername()
    {
        var settings = new AniListSettings { Username = "test" };
        Assert.True(settings.HasValidCredentials);
    }

    [Fact]
    public void Settings_HasValidCredentials_WithToken()
    {
        var settings = new AniListSettings { AccessToken = "token" };
        Assert.True(settings.HasValidCredentials);
    }

    [Fact]
    public void Settings_HasValidCredentials_NoCredentials()
    {
        var settings = new AniListSettings { Username = "", AccessToken = "" };
        Assert.False(settings.HasValidCredentials);
    }

    [Fact]
    public async Task Fetch_GraphQLError_HandlesGracefully()
    {
        var errorResponse = new AniListGraphQLResponse<AniListMediaListCollectionData>
        {
            Errors = new List<AniListError>
            {
                new() { Message = "User not found", Status = 404 }
            }
        };

        var handler = CreateMockHandler(errorResponse);
        var httpClient = new HttpClient(handler);

        var importList = CreateImportList(httpClient);
        var result = await importList.FetchAsync();

        // Should handle error gracefully — empty items, no crash
        Assert.Empty(result.Items);
    }

    [Fact]
    public async Task Fetch_TitleFallback_UsesRomajiWhenNoEnglish()
    {
        var graphqlResponse = new AniListGraphQLResponse<AniListMediaListCollectionData>
        {
            Data = new AniListMediaListCollectionData
            {
                MediaListCollection = new AniListMediaListCollection
                {
                    Lists = new List<AniListMediaListGroup>
                    {
                        new()
                        {
                            Entries = new List<AniListMediaListEntry>
                            {
                                new()
                                {
                                    Status = "CURRENT",
                                    Media = new AniListMedia
                                    {
                                        Id = 999,
                                        Title = new AniListTitle
                                        {
                                            English = null,
                                            Romaji = "Kimetsu no Yaiba",
                                            Native = "鬼滅の刃"
                                        },
                                        Type = "ANIME"
                                    }
                                }
                            }
                        }
                    },
                    HasNextChunk = false
                }
            }
        };

        var handler = CreateMockHandler(graphqlResponse);
        var httpClient = new HttpClient(handler);

        var settings = new AniListSettings
        {
            Username = "test",
            ImportAnimeList = true,
            ImportMangaList = false
        };

        var importList = CreateImportList(httpClient, settings);
        var result = await importList.FetchAsync();

        Assert.Single(result.Items);
        Assert.Equal("Kimetsu no Yaiba", result.Items[0].Title);
    }

    private static HttpMessageHandler CreateMockHandler<T>(T responseBody)
    {
        var json = JsonSerializer.Serialize(responseBody);
        var handler = new Mock<HttpMessageHandler>();
        handler.Protected()
            .Setup<Task<HttpResponseMessage>>(
                "SendAsync",
                ItExpr.IsAny<HttpRequestMessage>(),
                ItExpr.IsAny<CancellationToken>())
            .Returns(() => Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent(json)
            }));
        return handler.Object;
    }
}
