// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.ImportLists;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.ImportLists;

public class ImportListResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public string Implementation { get; set; } = string.Empty;
    public bool Enabled { get; set; }
    public bool EnableAuto { get; set; }
    public ImportListType ListType { get; set; }
    public MediaType MediaType { get; set; }
    public TimeSpan MinRefreshInterval { get; set; }
    public bool Monitor { get; set; }
    public int QualityProfileId { get; set; }
    public string RootFolderPath { get; set; } = string.Empty;
    public bool SearchOnAdd { get; set; }
    public string Settings { get; set; } = "{}";
    public List<string> Tags { get; set; } = new();
}

public class ImportListItemResource
{
    public int ListId { get; set; }
    public MediaType MediaType { get; set; }
    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }
    public int TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public int TvdbId { get; set; }
    public long GoodreadsId { get; set; }
    public string? Isbn { get; set; }
    public Guid MusicBrainzId { get; set; }
    public string? Artist { get; set; }
    public string? Album { get; set; }
    public string? Narrator { get; set; }
    public string? Asin { get; set; }
    public string? PodcastGuid { get; set; }
    public string? FeedUrl { get; set; }
}

public static class ImportListResourceMapper
{
    public static ImportListResource ToResource(this ImportListDefinition definition)
    {
        return new ImportListResource
        {
            Id = definition.Id,
            Name = definition.Name,
            Implementation = definition.Implementation,
            Enabled = definition.Enabled,
            EnableAuto = definition.EnableAuto,
            ListType = definition.ListType,
            MediaType = definition.MediaType,
            MinRefreshInterval = definition.MinRefreshInterval,
            Monitor = definition.Monitor,
            QualityProfileId = definition.QualityProfileId,
            RootFolderPath = definition.RootFolderPath,
            SearchOnAdd = definition.SearchOnAdd,
            Settings = definition.Settings,
            Tags = definition.Tags.Select(t => t.ToString()).ToList()
        };
    }

    public static ImportListDefinition ToDefinition(this ImportListResource resource)
    {
        return new ImportListDefinition
        {
            Id = resource.Id,
            Name = resource.Name,
            Implementation = resource.Implementation,
            Enabled = resource.Enabled,
            EnableAuto = resource.EnableAuto,
            ListType = resource.ListType,
            MediaType = resource.MediaType,
            MinRefreshInterval = resource.MinRefreshInterval,
            Monitor = resource.Monitor,
            QualityProfileId = resource.QualityProfileId,
            RootFolderPath = resource.RootFolderPath,
            SearchOnAdd = resource.SearchOnAdd,
            Settings = resource.Settings,
            Tags = resource.Tags.Select(t => int.TryParse(t, out var id) ? id : 0).Where(id => id > 0).ToHashSet()
        };
    }

    public static ImportListItemResource ToResource(this ImportListItem item)
    {
        return new ImportListItemResource
        {
            ListId = item.ListId,
            MediaType = item.MediaType,
            Title = item.Title,
            Year = item.Year,
            TmdbId = item.TmdbId,
            ImdbId = item.ImdbId,
            TvdbId = item.TvdbId,
            GoodreadsId = item.GoodreadsId,
            Isbn = item.Isbn,
            MusicBrainzId = item.MusicBrainzId,
            Artist = item.Artist,
            Album = item.Album,
            Narrator = item.Narrator,
            Asin = item.Asin,
            PodcastGuid = item.PodcastGuid,
            FeedUrl = item.FeedUrl
        };
    }
}
