// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaFiles;
using Mouseion.Core.MediaFiles.Audio;

namespace Mouseion.Core.Music;

public interface IAudioAnalysisService
{
    Task<AudioAnalysis?> GetAnalysisAsync(int trackId, CancellationToken ct = default);
    AudioAnalysis? GetAnalysis(int trackId);
    Task<Dictionary<int, AudioAnalysis>> GetAnalysisBatchAsync(IEnumerable<int> trackIds, CancellationToken ct = default);
    Dictionary<int, AudioAnalysis> GetAnalysisBatch(IEnumerable<int> trackIds);
}

public class AudioAnalysisService : IAudioAnalysisService
{
    private readonly IMusicFileRepository _musicFileRepository;
    private readonly IMusicFileAnalyzer _musicFileAnalyzer;
    private readonly IDynamicRangeAnalyzer _dynamicRangeAnalyzer;

    public AudioAnalysisService(
        IMusicFileRepository musicFileRepository,
        IMusicFileAnalyzer musicFileAnalyzer,
        IDynamicRangeAnalyzer dynamicRangeAnalyzer)
    {
        _musicFileRepository = musicFileRepository;
        _musicFileAnalyzer = musicFileAnalyzer;
        _dynamicRangeAnalyzer = dynamicRangeAnalyzer;
    }

    public async Task<AudioAnalysis?> GetAnalysisAsync(int trackId, CancellationToken ct = default)
    {
        var musicFiles = await _musicFileRepository.GetByTrackIdAsync(trackId, ct).ConfigureAwait(false);
        if (musicFiles.Count == 0)
        {
            return null;
        }

        var musicFile = musicFiles[0];
        if (string.IsNullOrEmpty(musicFile.RelativePath))
        {
            return null;
        }

        var fileInfo = await _musicFileAnalyzer.AnalyzeAsync(musicFile.RelativePath, ct).ConfigureAwait(false);
        if (fileInfo == null)
        {
            return null;
        }

        var dr = await _dynamicRangeAnalyzer.AnalyzeAsync(musicFile.RelativePath, ct).ConfigureAwait(false);

        return new AudioAnalysis
        {
            Format = fileInfo.Codec,
            SampleRate = fileInfo.SampleRate,
            BitDepth = fileInfo.BitsPerSample,
            Channels = fileInfo.Channels,
            DynamicRange = dr,
            ReplayGain = new ReplayGainInfo
            {
                TrackGain = fileInfo.ReplayGainTrackGain,
                TrackPeak = fileInfo.ReplayGainTrackPeak,
                AlbumGain = fileInfo.ReplayGainAlbumGain,
                AlbumPeak = fileInfo.ReplayGainAlbumPeak
            },
            Lossless = IsLosslessFormat(fileInfo.Codec),
            Transcoded = false
        };
    }

    public AudioAnalysis? GetAnalysis(int trackId)
    {
        var musicFiles = _musicFileRepository.GetByTrackId(trackId);
        if (musicFiles.Count == 0)
        {
            return null;
        }

        var musicFile = musicFiles[0];
        if (string.IsNullOrEmpty(musicFile.RelativePath))
        {
            return null;
        }

        var fileInfo = _musicFileAnalyzer.Analyze(musicFile.RelativePath);
        if (fileInfo == null)
        {
            return null;
        }

        var dr = _dynamicRangeAnalyzer.Analyze(musicFile.RelativePath);

        return new AudioAnalysis
        {
            Format = fileInfo.Codec,
            SampleRate = fileInfo.SampleRate,
            BitDepth = fileInfo.BitsPerSample,
            Channels = fileInfo.Channels,
            DynamicRange = dr,
            ReplayGain = new ReplayGainInfo
            {
                TrackGain = fileInfo.ReplayGainTrackGain,
                TrackPeak = fileInfo.ReplayGainTrackPeak,
                AlbumGain = fileInfo.ReplayGainAlbumGain,
                AlbumPeak = fileInfo.ReplayGainAlbumPeak
            },
            Lossless = IsLosslessFormat(fileInfo.Codec),
            Transcoded = false
        };
    }

    public async Task<Dictionary<int, AudioAnalysis>> GetAnalysisBatchAsync(IEnumerable<int> trackIds, CancellationToken ct = default)
    {
        var trackIdList = trackIds.ToList();
        var result = new Dictionary<int, AudioAnalysis>();

        foreach (var trackId in trackIdList)
        {
            var analysis = await GetAnalysisAsync(trackId, ct).ConfigureAwait(false);
            if (analysis != null)
            {
                result[trackId] = analysis;
            }
        }

        return result;
    }

    public Dictionary<int, AudioAnalysis> GetAnalysisBatch(IEnumerable<int> trackIds)
    {
        var trackIdList = trackIds.ToList();
        var result = new Dictionary<int, AudioAnalysis>();

        foreach (var trackId in trackIdList)
        {
            var analysis = GetAnalysis(trackId);
            if (analysis != null)
            {
                result[trackId] = analysis;
            }
        }

        return result;
    }

    private static bool IsLosslessFormat(string codec)
    {
        return codec switch
        {
            "FLAC" or "PCM" or "APE" or "WavPack" or "DSD" => true,
            _ => false
        };
    }
}

public class AudioAnalysis
{
    public string Format { get; set; } = string.Empty;
    public int SampleRate { get; set; }
    public int BitDepth { get; set; }
    public int Channels { get; set; }
    public int? DynamicRange { get; set; }
    public ReplayGainInfo? ReplayGain { get; set; }
    public bool Lossless { get; set; }
    public bool Transcoded { get; set; }
}

public class ReplayGainInfo
{
    public double? TrackGain { get; set; }
    public double? TrackPeak { get; set; }
    public double? AlbumGain { get; set; }
    public double? AlbumPeak { get; set; }
}
