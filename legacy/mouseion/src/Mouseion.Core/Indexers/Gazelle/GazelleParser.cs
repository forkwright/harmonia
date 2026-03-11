// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Indexers.Gazelle;

public class GazelleParser
{
    private readonly ILogger<GazelleParser> _logger;

    public GazelleParser(ILogger<GazelleParser> logger)
    {
        _logger = logger;
    }

    public List<GazelleRelease> ParseSearchResponse(string json)
    {
        var results = new List<GazelleRelease>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (!root.TryGetProperty("status", out var status) || status.GetString() != "success")
            {
                _logger.LogWarning("Gazelle response status is not success");
                return results;
            }

            if (!root.TryGetProperty("response", out var response) ||
                !response.TryGetProperty("results", out var resultsArray))
            {
                return results;
            }

            foreach (var group in resultsArray.EnumerateArray())
            {
                ParseReleaseGroup(group, results);
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing Gazelle search response");
        }

        return results;
    }

    private void ParseReleaseGroup(JsonElement group, List<GazelleRelease> results)
    {
        try
        {
            var artist = GetStringProperty(group, "artist") ?? "Unknown";
            var groupName = GetStringProperty(group, "groupName") ?? "Unknown";
            var groupYear = GetIntProperty(group, "groupYear");
            var cover = GetStringProperty(group, "cover");
            var tags = GetArrayProperty(group, "tags");

            if (group.TryGetProperty("torrents", out var torrents))
            {
                foreach (var torrent in torrents.EnumerateArray())
                {
                    var release = ParseTorrent(torrent, artist, groupName, groupYear, cover, tags);
                    if (release != null)
                    {
                        results.Add(release);
                    }
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogWarning(ex, "Error parsing Gazelle release group");
        }
    }

    private static GazelleRelease? ParseTorrent(
        JsonElement torrent,
        string artist,
        string groupName,
        int? groupYear,
        string? cover,
        List<string> tags)
    {
        var torrentId = GetIntProperty(torrent, "torrentId");
        if (torrentId == null)
        {
            return null;
        }

        var release = new GazelleRelease
        {
            TorrentId = torrentId.Value.ToString(),
            Artist = artist,
            Album = groupName,
            Year = groupYear,
            Cover = cover,
            Tags = tags,
            Format = GetStringProperty(torrent, "format") ?? string.Empty,
            Encoding = GetStringProperty(torrent, "encoding") ?? string.Empty,
            Media = GetStringProperty(torrent, "media") ?? string.Empty,
            Scene = GetBoolProperty(torrent, "scene"),
            HasLog = GetBoolProperty(torrent, "hasLog"),
            HasCue = GetBoolProperty(torrent, "hasCue"),
            LogScore = GetIntProperty(torrent, "logScore") ?? 0,
            Size = GetLongProperty(torrent, "size") ?? 0,
            Seeders = GetIntProperty(torrent, "seeders") ?? 0,
            Leechers = GetIntProperty(torrent, "leechers") ?? 0,
            Snatched = GetIntProperty(torrent, "snatched") ?? 0,
            IsFreeleech = GetBoolProperty(torrent, "isFreeleech"),
            IsNeutralLeech = GetBoolProperty(torrent, "isNeutralLeech"),
            Time = GetStringProperty(torrent, "time")
        };

        return release;
    }

    private static string? GetStringProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.String)
        {
            return property.GetString();
        }

        return null;
    }

    private static int? GetIntProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.Number)
            {
                return property.GetInt32();
            }
        }

        return null;
    }

    private static long? GetLongProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.Number)
            {
                return property.GetInt64();
            }
        }

        return null;
    }

    private static bool GetBoolProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            return property.ValueKind == JsonValueKind.True;
        }

        return false;
    }

    private static List<string> GetArrayProperty(JsonElement element, string propertyName)
    {
        var list = new List<string>();

        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in property.EnumerateArray())
            {
                if (item.ValueKind == JsonValueKind.String)
                {
                    var value = item.GetString();
                    if (!string.IsNullOrEmpty(value))
                    {
                        list.Add(value);
                    }
                }
            }
        }

        return list;
    }
}

public class GazelleRelease
{
    public string TorrentId { get; set; } = string.Empty;
    public string Artist { get; set; } = string.Empty;
    public string Album { get; set; } = string.Empty;
    public int? Year { get; set; }
    public string? Cover { get; set; }
    public List<string> Tags { get; set; } = new();
    public string Format { get; set; } = string.Empty;
    public string Encoding { get; set; } = string.Empty;
    public string Media { get; set; } = string.Empty;
    public bool Scene { get; set; }
    public bool HasLog { get; set; }
    public bool HasCue { get; set; }
    public int LogScore { get; set; }
    public long Size { get; set; }
    public int Seeders { get; set; }
    public int Leechers { get; set; }
    public int Snatched { get; set; }
    public bool IsFreeleech { get; set; }
    public bool IsNeutralLeech { get; set; }
    public string? Time { get; set; }
}
