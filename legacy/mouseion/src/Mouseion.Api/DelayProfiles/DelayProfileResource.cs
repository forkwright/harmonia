// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Mouseion.Core.Download.DelayProfiles;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Api.DelayProfiles;

public class DelayProfileResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public MediaType? MediaType { get; set; }
    public DownloadProtocol PreferredProtocol { get; set; }
    public int UsenetDelay { get; set; }
    public int TorrentDelay { get; set; }
    public int PreferredQualityWeight { get; set; }
    public bool BypassIfPreferredQuality { get; set; }
    public bool BypassIfPreferredWords { get; set; }
    public string? Tags { get; set; }
    public int Order { get; set; }
    public bool Enabled { get; set; }
}

public static class DelayProfileResourceMapper
{
    public static DelayProfileResource ToResource(this DelayProfile model) => new()
    {
        Id = model.Id,
        Name = model.Name,
        MediaType = model.MediaType,
        PreferredProtocol = model.PreferredProtocol,
        UsenetDelay = model.UsenetDelay,
        TorrentDelay = model.TorrentDelay,
        PreferredQualityWeight = model.PreferredQualityWeight,
        BypassIfPreferredQuality = model.BypassIfPreferredQuality,
        BypassIfPreferredWords = model.BypassIfPreferredWords,
        Tags = model.Tags,
        Order = model.Order,
        Enabled = model.Enabled
    };

    public static DelayProfile ToModel(this DelayProfileResource resource) => new()
    {
        Id = resource.Id,
        Name = resource.Name,
        MediaType = resource.MediaType,
        PreferredProtocol = resource.PreferredProtocol,
        UsenetDelay = resource.UsenetDelay,
        TorrentDelay = resource.TorrentDelay,
        PreferredQualityWeight = resource.PreferredQualityWeight,
        BypassIfPreferredQuality = resource.BypassIfPreferredQuality,
        BypassIfPreferredWords = resource.BypassIfPreferredWords,
        Tags = resource.Tags,
        Order = resource.Order,
        Enabled = resource.Enabled
    };
}
