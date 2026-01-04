// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Mouseion.Core.History;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Qualities;

namespace Mouseion.Api.History;

public class HistoryResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string SourceTitle { get; set; } = null!;
    public QualityModel Quality { get; set; } = null!;
    public DateTime Date { get; set; }
    public HistoryEventType EventType { get; set; }
    public Dictionary<string, object> Data { get; set; } = new();
    public string? DownloadId { get; set; }
}

public class PagedHistoryResource
{
    public int Page { get; set; }
    public int PageSize { get; set; }
    public string? SortKey { get; set; }
    public string? SortDirection { get; set; }
    public int TotalRecords { get; set; }
    public List<HistoryResource> Records { get; set; } = new();
}

public static class HistoryResourceMapper
{
    public static HistoryResource ToResource(this MediaItemHistory model)
    {
        var data = new Dictionary<string, object>();
        if (!string.IsNullOrWhiteSpace(model.Data) && model.Data != "{}")
        {
            try
            {
                data = JsonSerializer.Deserialize<Dictionary<string, object>>(model.Data) ?? new();
            }
            catch
            {
                data = new Dictionary<string, object>();
            }
        }

        return new HistoryResource
        {
            Id = model.Id,
            MediaItemId = model.MediaItemId,
            MediaType = model.MediaType,
            SourceTitle = model.SourceTitle,
            Quality = model.Quality,
            Date = model.Date,
            EventType = model.EventType,
            Data = data,
            DownloadId = model.DownloadId
        };
    }

    public static MediaItemHistory ToModel(this HistoryResource resource)
    {
        var dataJson = "{}";
        if (resource.Data.Count > 0)
        {
            dataJson = JsonSerializer.Serialize(resource.Data);
        }

        return new MediaItemHistory
        {
            Id = resource.Id,
            MediaItemId = resource.MediaItemId,
            MediaType = resource.MediaType,
            SourceTitle = resource.SourceTitle,
            Quality = resource.Quality,
            Date = resource.Date,
            EventType = resource.EventType,
            Data = dataJson,
            DownloadId = resource.DownloadId
        };
    }
}
