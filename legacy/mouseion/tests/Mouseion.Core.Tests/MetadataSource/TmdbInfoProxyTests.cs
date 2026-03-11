// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Reflection;
using System.Text.Json;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Core.Tests.MetadataSource;

public class TmdbInfoProxyTests
{
    private static void InvokeExtractGenreNames(JsonElement genresArray, List<string> genres)
    {
        var method = typeof(TmdbInfoProxy).GetMethod(
            "ExtractGenreNames",
            BindingFlags.NonPublic | BindingFlags.Static);
        method?.Invoke(null, new object[] { genresArray, genres });
    }

    private static void InvokeExtractGenreIds(JsonElement genreIds, List<string> genres)
    {
        var method = typeof(TmdbInfoProxy).GetMethod(
            "ExtractGenreIds",
            BindingFlags.NonPublic | BindingFlags.Static);
        method?.Invoke(null, new object[] { genreIds, genres });
    }

    [Fact]
    public void should_extract_genre_names_from_valid_array()
    {
        var json = """
        {
            "genres": [
                {"id": 28, "name": "Action"},
                {"id": 12, "name": "Adventure"},
                {"id": 878, "name": "Science Fiction"}
            ]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genres", out var genresArray);
        InvokeExtractGenreNames(genresArray, genres);

        Assert.Equal(3, genres.Count);
        Assert.Contains("Action", genres);
        Assert.Contains("Adventure", genres);
        Assert.Contains("Science Fiction", genres);
    }

    [Fact]
    public void should_skip_null_or_whitespace_genre_names()
    {
        var json = """
        {
            "genres": [
                {"id": 28, "name": "Action"},
                {"id": 12, "name": ""},
                {"id": 878, "name": "   "},
                {"id": 99}
            ]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genres", out var genresArray);
        InvokeExtractGenreNames(genresArray, genres);

        Assert.Single(genres);
        Assert.Equal("Action", genres[0]);
    }

    [Fact]
    public void should_handle_empty_genre_array()
    {
        var json = """
        {
            "genres": []
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genres", out var genresArray);
        InvokeExtractGenreNames(genresArray, genres);

        Assert.Empty(genres);
    }

    [Fact]
    public void should_handle_genre_objects_without_name_property()
    {
        var json = """
        {
            "genres": [
                {"id": 28},
                {"other": "value"}
            ]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genres", out var genresArray);
        InvokeExtractGenreNames(genresArray, genres);

        Assert.Empty(genres);
    }

    [Fact]
    public void should_handle_genre_name_as_non_string()
    {
        var json = """
        {
            "genres": [
                {"id": 28, "name": 123},
                {"id": 12, "name": true}
            ]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genres", out var genresArray);
        InvokeExtractGenreNames(genresArray, genres);

        Assert.Empty(genres);
    }

    [Fact]
    public void should_extract_genre_ids_from_valid_array()
    {
        var json = """
        {
            "genre_ids": [28, 12, 878]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Equal(3, genres.Count);
        Assert.Contains("28", genres);
        Assert.Contains("12", genres);
        Assert.Contains("878", genres);
    }

    [Fact]
    public void should_handle_empty_genre_ids_array()
    {
        var json = """
        {
            "genre_ids": []
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Empty(genres);
    }

    [Fact]
    public void should_skip_non_numeric_genre_ids()
    {
        var json = """
        {
            "genre_ids": [28, "string", true, null, 12]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Equal(2, genres.Count);
        Assert.Contains("28", genres);
        Assert.Contains("12", genres);
    }

    [Fact]
    public void should_handle_zero_genre_id()
    {
        var json = """
        {
            "genre_ids": [0, 28, 12]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Equal(3, genres.Count);
        Assert.Contains("0", genres);
        Assert.Contains("28", genres);
        Assert.Contains("12", genres);
    }

    [Fact]
    public void should_handle_negative_genre_id()
    {
        var json = """
        {
            "genre_ids": [-1, 28]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Equal(2, genres.Count);
        Assert.Contains("-1", genres);
        Assert.Contains("28", genres);
    }

    [Fact]
    public void should_handle_large_genre_id()
    {
        var json = """
        {
            "genre_ids": [2147483647, 28]
        }
        """;

        var element = JsonDocument.Parse(json).RootElement;
        var genres = new List<string>();

        element.TryGetProperty("genre_ids", out var genreIds);
        InvokeExtractGenreIds(genreIds, genres);

        Assert.Equal(2, genres.Count);
        Assert.Contains("2147483647", genres);
        Assert.Contains("28", genres);
    }
}
