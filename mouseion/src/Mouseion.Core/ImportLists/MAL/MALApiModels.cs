// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.ImportLists.MAL;

/// <summary>
/// MAL API v2 models. Docs: https://myanimelist.net/apiconfig/references/api/v2
/// </summary>

public class MALPagedResponse<T>
{
    [JsonPropertyName("data")]
    public List<T> Data { get; set; } = new();

    [JsonPropertyName("paging")]
    public MALPaging? Paging { get; set; }
}

public class MALPaging
{
    [JsonPropertyName("previous")]
    public string? Previous { get; set; }

    [JsonPropertyName("next")]
    public string? Next { get; set; }
}

public class MALAnimeListItem
{
    [JsonPropertyName("node")]
    public MALAnimeNode Node { get; set; } = new();

    [JsonPropertyName("list_status")]
    public MALAnimeListStatus? ListStatus { get; set; }
}

public class MALAnimeNode
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("title")]
    public string Title { get; set; } = string.Empty;

    [JsonPropertyName("main_picture")]
    public MALPicture? MainPicture { get; set; }

    [JsonPropertyName("start_date")]
    public string? StartDate { get; set; }

    [JsonPropertyName("media_type")]
    public string? MediaType { get; set; }

    [JsonPropertyName("num_episodes")]
    public int NumEpisodes { get; set; }
}

public class MALAnimeListStatus
{
    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("score")]
    public int Score { get; set; }

    [JsonPropertyName("num_episodes_watched")]
    public int NumEpisodesWatched { get; set; }

    [JsonPropertyName("is_rewatching")]
    public bool IsRewatching { get; set; }

    [JsonPropertyName("updated_at")]
    public string? UpdatedAt { get; set; }

    [JsonPropertyName("start_date")]
    public string? StartDate { get; set; }

    [JsonPropertyName("finish_date")]
    public string? FinishDate { get; set; }
}

public class MALMangaListItem
{
    [JsonPropertyName("node")]
    public MALMangaNode Node { get; set; } = new();

    [JsonPropertyName("list_status")]
    public MALMangaListStatus? ListStatus { get; set; }
}

public class MALMangaNode
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("title")]
    public string Title { get; set; } = string.Empty;

    [JsonPropertyName("main_picture")]
    public MALPicture? MainPicture { get; set; }

    [JsonPropertyName("start_date")]
    public string? StartDate { get; set; }

    [JsonPropertyName("media_type")]
    public string? MediaType { get; set; }

    [JsonPropertyName("num_chapters")]
    public int NumChapters { get; set; }

    [JsonPropertyName("num_volumes")]
    public int NumVolumes { get; set; }
}

public class MALMangaListStatus
{
    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("score")]
    public int Score { get; set; }

    [JsonPropertyName("num_chapters_read")]
    public int NumChaptersRead { get; set; }

    [JsonPropertyName("num_volumes_read")]
    public int NumVolumesRead { get; set; }

    [JsonPropertyName("is_rereading")]
    public bool IsRereading { get; set; }

    [JsonPropertyName("updated_at")]
    public string? UpdatedAt { get; set; }

    [JsonPropertyName("start_date")]
    public string? StartDate { get; set; }

    [JsonPropertyName("finish_date")]
    public string? FinishDate { get; set; }
}

public class MALPicture
{
    [JsonPropertyName("medium")]
    public string? Medium { get; set; }

    [JsonPropertyName("large")]
    public string? Large { get; set; }
}

public class MALTokenResponse
{
    [JsonPropertyName("access_token")]
    public string AccessToken { get; set; } = string.Empty;

    [JsonPropertyName("refresh_token")]
    public string RefreshToken { get; set; } = string.Empty;

    [JsonPropertyName("expires_in")]
    public int ExpiresIn { get; set; }

    [JsonPropertyName("token_type")]
    public string TokenType { get; set; } = string.Empty;
}
