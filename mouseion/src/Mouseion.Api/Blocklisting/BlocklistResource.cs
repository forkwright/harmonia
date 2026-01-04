// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Blocklisting;
using Mouseion.Core.Qualities;

namespace Mouseion.Api.Blocklisting;

public class BlocklistResource
{
    public int Id { get; set; }
    public int MediaItemId { get; set; }
    public string SourceTitle { get; set; } = null!;
    public QualityModel Quality { get; set; } = null!;
    public DateTime Date { get; set; }
    public DateTime? PublishedDate { get; set; }
    public long? Size { get; set; }
    public DownloadProtocol Protocol { get; set; }
    public string? Indexer { get; set; }
    public string? Message { get; set; }
}

public static class BlocklistResourceMapper
{
    public static BlocklistResource ToResource(this Core.Blocklisting.Blocklist model)
    {
        return new BlocklistResource
        {
            Id = model.Id,
            MediaItemId = model.MediaItemId,
            SourceTitle = model.SourceTitle,
            Quality = model.Quality,
            Date = model.Date,
            PublishedDate = model.PublishedDate,
            Size = model.Size,
            Protocol = model.Protocol,
            Indexer = model.Indexer,
            Message = model.Message
        };
    }

    public static Core.Blocklisting.Blocklist ToModel(this BlocklistResource resource)
    {
        return new Core.Blocklisting.Blocklist
        {
            Id = resource.Id,
            MediaItemId = resource.MediaItemId,
            SourceTitle = resource.SourceTitle,
            Quality = resource.Quality,
            Date = resource.Date,
            PublishedDate = resource.PublishedDate,
            Size = resource.Size,
            Protocol = resource.Protocol,
            Indexer = resource.Indexer,
            Message = resource.Message
        };
    }
}
