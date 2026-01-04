// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Filtering;
using Mouseion.Core.Music;

namespace Mouseion.Core.Library;

public interface ILibraryFilterService
{
    Task<FilterResult> FilterTracksAsync(FilterRequest request, CancellationToken ct = default);
    FilterResult FilterTracks(FilterRequest request);
}

public class FilterResult
{
    public List<Track> Tracks { get; set; } = new();
    public int TotalCount { get; set; }
    public int Page { get; set; }
    public int PageSize { get; set; }
    public FilterSummary? Summary { get; set; }
}

public class LibraryFilterService : ILibraryFilterService
{
    private readonly ITrackRepository _trackRepository;
    private readonly IFilterQueryBuilder _queryBuilder;
    private readonly IMusicFileRepository _musicFileRepository;

    public LibraryFilterService(
        ITrackRepository trackRepository,
        IFilterQueryBuilder queryBuilder,
        IMusicFileRepository musicFileRepository)
    {
        _trackRepository = trackRepository;
        _queryBuilder = queryBuilder;
        _musicFileRepository = musicFileRepository;
    }

    public async Task<FilterResult> FilterTracksAsync(FilterRequest request, CancellationToken ct = default)
    {
        ValidateRequest(request);

        var totalCount = await _trackRepository.CountAsync(ct).ConfigureAwait(false);
        var tracks = await _trackRepository.FilterAsync(request, ct).ConfigureAwait(false);

        var summary = await ComputeSummaryAsync(tracks, ct).ConfigureAwait(false);

        return new FilterResult
        {
            Tracks = tracks,
            Page = request.Page,
            PageSize = request.PageSize,
            TotalCount = totalCount,
            Summary = summary
        };
    }

    public FilterResult FilterTracks(FilterRequest request)
    {
        ValidateRequest(request);

        var totalCount = _trackRepository.Count();
        var tracks = _trackRepository.Filter(request);

        var summary = ComputeSummary(tracks);

        return new FilterResult
        {
            Tracks = tracks,
            Page = request.Page,
            PageSize = request.PageSize,
            TotalCount = totalCount,
            Summary = summary
        };
    }

    private void ValidateRequest(FilterRequest request)
    {
        if (request.Conditions.Count == 0)
        {
            throw new ArgumentException("At least one filter condition is required");
        }

        if (request.Page < 1)
        {
            throw new ArgumentException("Page must be >= 1");
        }

        if (request.PageSize < 1 || request.PageSize > 250)
        {
            throw new ArgumentException("PageSize must be between 1 and 250");
        }

        foreach (var condition in request.Conditions)
        {
            if (string.IsNullOrWhiteSpace(condition.Field))
            {
                throw new ArgumentException("Filter field cannot be empty");
            }

            if (!_queryBuilder.IsValidField(condition.Field))
            {
                throw new ArgumentException($"Field '{condition.Field}' is not allowed for filtering");
            }
        }
    }

    private async Task<FilterSummary> ComputeSummaryAsync(List<Track> tracks, CancellationToken ct = default)
    {
        if (tracks.Count == 0)
        {
            return new FilterSummary();
        }

        var trackIds = tracks.Select(t => t.Id).ToList();
        var musicFiles = new List<MusicFile>();

        foreach (var trackId in trackIds)
        {
            var files = await _musicFileRepository.GetByTrackIdAsync(trackId, ct).ConfigureAwait(false);
            musicFiles.AddRange(files);
        }

        return ComputeSummaryFromFiles(musicFiles);
    }

    private FilterSummary ComputeSummary(List<Track> tracks)
    {
        if (tracks.Count == 0)
        {
            return new FilterSummary();
        }

        var trackIds = tracks.Select(t => t.Id).ToList();
        var musicFiles = new List<MusicFile>();

        foreach (var trackId in trackIds)
        {
            var files = _musicFileRepository.GetByTrackId(trackId);
            musicFiles.AddRange(files);
        }

        return ComputeSummaryFromFiles(musicFiles);
    }

    private static FilterSummary ComputeSummaryFromFiles(List<MusicFile> musicFiles)
    {
        if (musicFiles.Count == 0)
        {
            return new FilterSummary();
        }

        var dynamicRanges = musicFiles
            .Where(f => f.DynamicRange.HasValue)
            .Select(f => f.DynamicRange!.Value)
            .ToList();

        var formatDistribution = musicFiles
            .Where(f => !string.IsNullOrEmpty(f.AudioFormat))
            .GroupBy(f => f.AudioFormat!)
            .ToDictionary(g => g.Key, g => g.Count());

        var sampleRateDistribution = musicFiles
            .Where(f => f.SampleRate.HasValue)
            .GroupBy(f => f.SampleRate!.Value)
            .ToDictionary(g => g.Key, g => g.Count());

        var losslessCount = musicFiles.Count(f => f.Lossless);

        return new FilterSummary
        {
            AvgDynamicRange = dynamicRanges.Count > 0 ? dynamicRanges.Average() : null,
            FormatDistribution = formatDistribution,
            SampleRateDistribution = sampleRateDistribution,
            LosslessCount = losslessCount
        };
    }
}
