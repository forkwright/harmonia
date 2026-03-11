// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;
using Mouseion.Core.SmartLists;

namespace Mouseion.Api.SmartLists;

public class SmartListResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public SmartListSource Source { get; set; }
    public MediaType MediaType { get; set; }
    public string QueryParametersJson { get; set; } = "{}";
    public int QualityProfileId { get; set; }
    public string RootFolderPath { get; set; } = string.Empty;
    public SmartListRefreshInterval RefreshInterval { get; set; }
    public bool SearchOnAdd { get; set; }
    public bool Enabled { get; set; }
    public int MaxItemsPerRefresh { get; set; }
    public int? MinimumRating { get; set; }
    public int? MinYear { get; set; }
    public int? MaxYear { get; set; }
    public string? ExcludeGenres { get; set; }
    public string? Language { get; set; }
    public string? Tags { get; set; }
    public int ItemsAdded { get; set; }
    public DateTime? LastRefreshed { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class SmartListMatchResource
{
    public int Id { get; set; }
    public int SmartListId { get; set; }
    public string ExternalId { get; set; } = string.Empty;
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int? Rating { get; set; }
    public SmartListMatchStatus Status { get; set; }
    public int? MediaItemId { get; set; }
    public string? MetadataJson { get; set; }
    public DateTime DiscoveredAt { get; set; }
    public DateTime? AddedAt { get; set; }
}

public static class SmartListResourceMapper
{
    public static SmartListResource ToResource(this SmartList model) => new()
    {
        Id = model.Id,
        Name = model.Name,
        Source = model.Source,
        MediaType = model.MediaType,
        QueryParametersJson = model.QueryParametersJson,
        QualityProfileId = model.QualityProfileId,
        RootFolderPath = model.RootFolderPath,
        RefreshInterval = model.RefreshInterval,
        SearchOnAdd = model.SearchOnAdd,
        Enabled = model.Enabled,
        MaxItemsPerRefresh = model.MaxItemsPerRefresh,
        MinimumRating = model.MinimumRating,
        MinYear = model.MinYear,
        MaxYear = model.MaxYear,
        ExcludeGenres = model.ExcludeGenres,
        Language = model.Language,
        Tags = model.Tags,
        ItemsAdded = model.ItemsAdded,
        LastRefreshed = model.LastRefreshed,
        CreatedAt = model.CreatedAt,
        UpdatedAt = model.UpdatedAt
    };

    public static SmartList ToModel(this SmartListResource resource) => new()
    {
        Id = resource.Id,
        Name = resource.Name,
        Source = resource.Source,
        MediaType = resource.MediaType,
        QueryParametersJson = resource.QueryParametersJson,
        QualityProfileId = resource.QualityProfileId,
        RootFolderPath = resource.RootFolderPath,
        RefreshInterval = resource.RefreshInterval,
        SearchOnAdd = resource.SearchOnAdd,
        Enabled = resource.Enabled,
        MaxItemsPerRefresh = resource.MaxItemsPerRefresh,
        MinimumRating = resource.MinimumRating,
        MinYear = resource.MinYear,
        MaxYear = resource.MaxYear,
        ExcludeGenres = resource.ExcludeGenres,
        Language = resource.Language,
        Tags = resource.Tags
    };

    public static SmartListMatchResource ToResource(this SmartListMatch model) => new()
    {
        Id = model.Id,
        SmartListId = model.SmartListId,
        ExternalId = model.ExternalId,
        MediaType = model.MediaType,
        Title = model.Title,
        Year = model.Year,
        Rating = model.Rating,
        Status = model.Status,
        MediaItemId = model.MediaItemId,
        MetadataJson = model.MetadataJson,
        DiscoveredAt = model.DiscoveredAt,
        AddedAt = model.AddedAt
    };
}
