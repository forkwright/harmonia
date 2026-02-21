// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.ImportLists.AniList;

/// <summary>
/// AniList GraphQL response models.
/// Docs: https://anilist.gitbook.io/anilist-apiv2-docs/
/// </summary>

public class AniListGraphQLResponse<T>
{
    [JsonPropertyName("data")]
    public T? Data { get; set; }

    [JsonPropertyName("errors")]
    public List<AniListError>? Errors { get; set; }
}

public class AniListError
{
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;

    [JsonPropertyName("status")]
    public int Status { get; set; }
}

public class AniListMediaListCollectionData
{
    [JsonPropertyName("MediaListCollection")]
    public AniListMediaListCollection? MediaListCollection { get; set; }
}

public class AniListMediaListCollection
{
    [JsonPropertyName("lists")]
    public List<AniListMediaListGroup> Lists { get; set; } = new();

    [JsonPropertyName("hasNextChunk")]
    public bool HasNextChunk { get; set; }
}

public class AniListMediaListGroup
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("status")]
    public string? Status { get; set; }

    [JsonPropertyName("entries")]
    public List<AniListMediaListEntry> Entries { get; set; } = new();
}

public class AniListMediaListEntry
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("score")]
    public double Score { get; set; }

    [JsonPropertyName("progress")]
    public int Progress { get; set; }

    [JsonPropertyName("progressVolumes")]
    public int? ProgressVolumes { get; set; }

    [JsonPropertyName("startedAt")]
    public AniListFuzzyDate? StartedAt { get; set; }

    [JsonPropertyName("completedAt")]
    public AniListFuzzyDate? CompletedAt { get; set; }

    [JsonPropertyName("updatedAt")]
    public long? UpdatedAt { get; set; }

    [JsonPropertyName("media")]
    public AniListMedia? Media { get; set; }
}

public class AniListMedia
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("idMal")]
    public int? IdMal { get; set; }

    [JsonPropertyName("title")]
    public AniListTitle? Title { get; set; }

    [JsonPropertyName("type")]
    public string Type { get; set; } = string.Empty;

    [JsonPropertyName("format")]
    public string? Format { get; set; }

    [JsonPropertyName("startDate")]
    public AniListFuzzyDate? StartDate { get; set; }

    [JsonPropertyName("episodes")]
    public int? Episodes { get; set; }

    [JsonPropertyName("chapters")]
    public int? Chapters { get; set; }

    [JsonPropertyName("volumes")]
    public int? Volumes { get; set; }
}

public class AniListTitle
{
    [JsonPropertyName("romaji")]
    public string? Romaji { get; set; }

    [JsonPropertyName("english")]
    public string? English { get; set; }

    [JsonPropertyName("native")]
    public string? Native { get; set; }
}

public class AniListFuzzyDate
{
    [JsonPropertyName("year")]
    public int? Year { get; set; }

    [JsonPropertyName("month")]
    public int? Month { get; set; }

    [JsonPropertyName("day")]
    public int? Day { get; set; }

    public DateTime? ToDateTime()
    {
        if (!Year.HasValue) return null;
        try
        {
            return new DateTime(Year.Value, Month ?? 1, Day ?? 1);
        }
        catch
        {
            return null;
        }
    }
}
