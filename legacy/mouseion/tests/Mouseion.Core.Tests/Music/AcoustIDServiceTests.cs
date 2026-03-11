// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Music;

namespace Mouseion.Core.Tests.Music;

public class AcoustIDServiceTests
{
    private readonly Mock<ILogger<AcoustIDService>> _mockLogger;
    private readonly AcoustIDService _service;

    public AcoustIDServiceTests()
    {
        _mockLogger = new Mock<ILogger<AcoustIDService>>();
        var mockHttpClient = new Mock<Common.Http.IHttpClient>();
        _service = new AcoustIDService(mockHttpClient.Object, _mockLogger.Object);
    }

    [Fact]
    public void IsValidResponse_WithStatusOk_ReturnsTrue()
    {
        var json = """{"status":"ok","results":[]}""";
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        var method = typeof(AcoustIDService).GetMethod("IsValidResponse",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (bool)method!.Invoke(_service, new object[] { root })!;

        Assert.True(result);
    }

    [Theory]
    [InlineData("""{"status":"error"}""")]
    [InlineData("""{"status":"failed"}""")]
    [InlineData("""{"foo":"bar"}""")]
    public void IsValidResponse_WithNonOkStatus_ReturnsFalse(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;

        var method = typeof(AcoustIDService).GetMethod("IsValidResponse",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = (bool)method!.Invoke(_service, new object[] { root })!;

        Assert.False(result);
    }

    [Fact]
    public void BuildAcoustIDResult_WithValidRecordings_CreatesResult()
    {
        var json = """
        {
            "id": "test-id",
            "score": 0.95,
            "recordings": [
                {
                    "id": "rec-123",
                    "title": "Test Song",
                    "artists": [
                        {"name": "Artist One"},
                        {"name": "Artist Two"}
                    ]
                }
            ]
        }
        """;

        using var doc = JsonDocument.Parse(json);
        var resultElement = doc.RootElement;

        var method = typeof(AcoustIDService).GetMethod("BuildAcoustIDResult",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (AcoustIDResult)method!.Invoke(null, new object[] { resultElement })!;

        Assert.NotNull(result);
        Assert.Single(result.Recordings);
        Assert.Equal("rec-123", result.Recordings[0].Id);
        Assert.Equal("Test Song", result.Recordings[0].Title);
        Assert.Equal(2, result.Recordings[0].Artists.Count);
        Assert.Contains("Artist One", result.Recordings[0].Artists);
        Assert.Contains("Artist Two", result.Recordings[0].Artists);
        Assert.Equal(0.95, result.Recordings[0].Score);
    }

    [Fact]
    public void BuildAcoustIDResult_WithNoRecordings_ReturnsEmptyList()
    {
        var json = """{"id": "test-id", "score": 0.85}""";

        using var doc = JsonDocument.Parse(json);
        var resultElement = doc.RootElement;

        var method = typeof(AcoustIDService).GetMethod("BuildAcoustIDResult",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (AcoustIDResult)method!.Invoke(null, new object[] { resultElement })!;

        Assert.NotNull(result);
        Assert.Empty(result.Recordings);
    }

    [Fact]
    public void ParseRecording_WithAllFields_ExtractsCorrectly()
    {
        var recordingJson = """
        {
            "id": "rec-456",
            "title": "Another Song",
            "artists": [{"name": "Solo Artist"}]
        }
        """;

        var parentJson = """{"score": 0.88}""";

        using var recordingDoc = JsonDocument.Parse(recordingJson);
        using var parentDoc = JsonDocument.Parse(parentJson);

        var method = typeof(AcoustIDService).GetMethod("ParseRecording",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (AcoustIDRecording)method!.Invoke(null,
            new object[] { recordingDoc.RootElement, parentDoc.RootElement })!;

        Assert.Equal("rec-456", result.Id);
        Assert.Equal("Another Song", result.Title);
        Assert.Single(result.Artists);
        Assert.Equal("Solo Artist", result.Artists[0]);
        Assert.Equal(0.88, result.Score);
    }

    [Fact]
    public void ParseRecording_WithMissingFields_HandlesGracefully()
    {
        var recordingJson = """{}""";
        var parentJson = """{}""";

        using var recordingDoc = JsonDocument.Parse(recordingJson);
        using var parentDoc = JsonDocument.Parse(parentJson);

        var method = typeof(AcoustIDService).GetMethod("ParseRecording",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (AcoustIDRecording)method!.Invoke(null,
            new object[] { recordingDoc.RootElement, parentDoc.RootElement })!;

        Assert.Equal(string.Empty, result.Id);
        Assert.Null(result.Title);
        Assert.Empty(result.Artists);
        Assert.Equal(0.0, result.Score);
    }

    [Fact]
    public void ExtractArtistNames_WithMultipleArtists_ExtractsAll()
    {
        var json = """
        [
            {"name": "Artist 1"},
            {"name": "Artist 2"},
            {"name": "Artist 3"}
        ]
        """;

        using var doc = JsonDocument.Parse(json);
        var artistList = new List<string>();

        var method = typeof(AcoustIDService).GetMethod("ExtractArtistNames",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        method!.Invoke(null, new object[] { doc.RootElement, artistList });

        Assert.Equal(3, artistList.Count);
        Assert.Equal("Artist 1", artistList[0]);
        Assert.Equal("Artist 2", artistList[1]);
        Assert.Equal("Artist 3", artistList[2]);
    }

    [Fact]
    public void ExtractArtistNames_WithEmptyNames_SkipsThem()
    {
        var json = """
        [
            {"name": "Valid Artist"},
            {"name": ""},
            {"name": "Another Valid"},
            {"other": "field"}
        ]
        """;

        using var doc = JsonDocument.Parse(json);
        var artistList = new List<string>();

        var method = typeof(AcoustIDService).GetMethod("ExtractArtistNames",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        method!.Invoke(null, new object[] { doc.RootElement, artistList });

        Assert.Equal(2, artistList.Count);
        Assert.Equal("Valid Artist", artistList[0]);
        Assert.Equal("Another Valid", artistList[1]);
    }

    [Fact]
    public void ExtractArtistNames_WithEmptyArray_ReturnsEmpty()
    {
        var json = """[]""";

        using var doc = JsonDocument.Parse(json);
        var artistList = new List<string>();

        var method = typeof(AcoustIDService).GetMethod("ExtractArtistNames",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        method!.Invoke(null, new object[] { doc.RootElement, artistList });

        Assert.Empty(artistList);
    }
}
