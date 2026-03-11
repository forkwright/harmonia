// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.ImportLists;

/// <summary>
/// Cross-references music imports from Last.fm and ListenBrainz to avoid double-import.
/// Users often have both — match by MusicBrainz ID (authoritative), fall back to artist+album.
/// When merging, prefers the item with MusicBrainz IDs (ListenBrainz usually wins).
/// </summary>
public interface IMusicCrossReferenceService
{
    List<ImportListItem> DeduplicateAcrossSources(List<ImportListItem> items);
}

public class MusicCrossReferenceService : IMusicCrossReferenceService
{
    private readonly ILogger<MusicCrossReferenceService> _logger;

    public MusicCrossReferenceService(ILogger<MusicCrossReferenceService> logger)
    {
        _logger = logger;
    }

    public List<ImportListItem> DeduplicateAcrossSources(List<ImportListItem> items)
    {
        var musicItems = items
            .Where(i => i.MediaType == MediaTypes.MediaType.Music)
            .ToList();

        var nonMusicItems = items
            .Where(i => i.MediaType != MediaTypes.MediaType.Music)
            .ToList();

        if (musicItems.Count == 0)
            return items;

        // Phase 1: Group by MusicBrainz ID
        var mbidGroups = new Dictionary<Guid, List<ImportListItem>>();
        var noMbid = new List<ImportListItem>();

        foreach (var item in musicItems)
        {
            if (item.MusicBrainzId != Guid.Empty)
            {
                if (!mbidGroups.ContainsKey(item.MusicBrainzId))
                    mbidGroups[item.MusicBrainzId] = new List<ImportListItem>();
                mbidGroups[item.MusicBrainzId].Add(item);
            }
            else
            {
                noMbid.Add(item);
            }
        }

        var result = new List<ImportListItem>();

        // Merge MBID groups
        foreach (var (mbid, group) in mbidGroups)
        {
            var best = PickBest(group);
            result.Add(best);

            if (group.Count > 1)
            {
                _logger.LogDebug(
                    "Music cross-reference: merged {Count} sources for MBID {Mbid} ({Title})",
                    group.Count, mbid, best.Title);
            }
        }

        // Phase 2: For items without MBID, check if they match an existing MBID item
        foreach (var item in noMbid)
        {
            var existing = result.FirstOrDefault(r =>
                FuzzyArtistAlbumMatch(r, item));

            if (existing != null)
            {
                // Already have this from a source with better data (MBID)
                _logger.LogDebug(
                    "Music cross-reference: dropped duplicate (fuzzy match) for {Artist} - {Title}",
                    item.Artist, item.Title);
                continue;
            }

            result.Add(item);
        }

        nonMusicItems.AddRange(result);
        return nonMusicItems;
    }

    private static ImportListItem PickBest(List<ImportListItem> group)
    {
        return group.OrderByDescending(MetadataScore).First();
    }

    private static int MetadataScore(ImportListItem item)
    {
        var score = 0;
        if (item.MusicBrainzId != Guid.Empty) score += 3; // MBID is most valuable
        if (item.UserRating.HasValue && item.UserRating.Value > 0) score += 2;
        if (item.WatchedAt.HasValue) score += 1;
        if (!string.IsNullOrEmpty(item.Album)) score += 1;
        if (!string.IsNullOrEmpty(item.Artist)) score += 1;
        // Prefer ListenBrainz data (has native MBIDs)
        if (item.ImportSource?.StartsWith("listenbrainz") == true) score += 1;
        return score;
    }

    private static bool FuzzyArtistAlbumMatch(ImportListItem a, ImportListItem b)
    {
        var artistMatch = Normalize(a.Artist ?? "") == Normalize(b.Artist ?? "");
        var titleMatch = Normalize(a.Title) == Normalize(b.Title);

        return artistMatch && titleMatch;
    }

    private static string Normalize(string value)
    {
        return new string(value.ToLowerInvariant()
            .Where(c => char.IsLetterOrDigit(c) || c == ' ')
            .ToArray())
            .Trim();
    }
}
