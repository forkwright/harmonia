// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Music;

namespace Mouseion.Api.Tracks;

[ApiController]
[Route("api/v3/tracks")]
[Authorize]
public class TracksController : ControllerBase
{
    private readonly IAudioAnalysisService _audioAnalysisService;
    private readonly ITrackRepository _trackRepository;

    public TracksController(
        IAudioAnalysisService audioAnalysisService,
        ITrackRepository trackRepository)
    {
        _audioAnalysisService = audioAnalysisService;
        _trackRepository = trackRepository;
    }

    [HttpGet("{id:int}/audio-analysis")]
    public async Task<ActionResult<AudioAnalysisResource>> GetAudioAnalysis(int id, CancellationToken ct = default)
    {
        var analysis = await _audioAnalysisService.GetAnalysisAsync(id, ct).ConfigureAwait(false);
        if (analysis == null)
        {
            return NotFound(new { error = $"Track {id} not found or has no associated music file" });
        }

        return Ok(ToResource(analysis));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<BatchTracksResource>> GetTracksBatch([FromBody][Required] BatchTrackRequest request, CancellationToken ct = default)
    {
        if (request?.TrackIds == null || request.TrackIds.Count == 0)
        {
            return BadRequest(new { error = "trackIds array is required and cannot be empty" });
        }

        if (request.TrackIds.Count > 100)
        {
            return BadRequest(new { error = "trackIds array cannot exceed 100 items" });
        }

        var tracks = await _trackRepository.GetByIdsAsync(request.TrackIds, ct).ConfigureAwait(false);
        var analyses = await _audioAnalysisService.GetAnalysisBatchAsync(request.TrackIds, ct).ConfigureAwait(false);

        var trackResources = tracks.Select(track => new TrackWithAudioResource
        {
            Id = track.Id,
            AlbumId = track.AlbumId,
            ArtistId = track.ArtistId,
            Title = track.Title,
            ForeignTrackId = track.ForeignTrackId,
            MusicBrainzId = track.MusicBrainzId,
            TrackNumber = track.TrackNumber,
            DiscNumber = track.DiscNumber,
            DurationSeconds = track.DurationSeconds,
            Explicit = track.Explicit,
            Monitored = track.Monitored,
            QualityProfileId = track.QualityProfileId,
            Added = track.Added,
            AudioAnalysis = analyses.ContainsKey(track.Id) ? ToResource(analyses[track.Id]) : null
        }).ToList();

        return Ok(new BatchTracksResource { Tracks = trackResources });
    }

    private static AudioAnalysisResource ToResource(AudioAnalysis analysis)
    {
        return new AudioAnalysisResource
        {
            Format = analysis.Format,
            SampleRate = analysis.SampleRate,
            BitDepth = analysis.BitDepth,
            Channels = analysis.Channels,
            DynamicRange = analysis.DynamicRange,
            ReplayGain = analysis.ReplayGain != null ? new ReplayGainResource
            {
                TrackGain = analysis.ReplayGain.TrackGain,
                TrackPeak = analysis.ReplayGain.TrackPeak,
                AlbumGain = analysis.ReplayGain.AlbumGain,
                AlbumPeak = analysis.ReplayGain.AlbumPeak
            } : null,
            Lossless = analysis.Lossless,
            Transcoded = analysis.Transcoded
        };
    }
}

public class AudioAnalysisResource
{
    public string Format { get; set; } = string.Empty;
    public int SampleRate { get; set; }
    public int BitDepth { get; set; }
    public int Channels { get; set; }
    public int? DynamicRange { get; set; }
    public ReplayGainResource? ReplayGain { get; set; }
    public bool Lossless { get; set; }
    public bool Transcoded { get; set; }
}

public class ReplayGainResource
{
    public double? TrackGain { get; set; }
    public double? TrackPeak { get; set; }
    public double? AlbumGain { get; set; }
    public double? AlbumPeak { get; set; }
}

public class BatchTrackRequest
{
    public List<int> TrackIds { get; set; } = new();
}

public class TrackWithAudioResource
{
    public int Id { get; set; }
    public int? AlbumId { get; set; }
    public int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? ForeignTrackId { get; set; }
    public string? MusicBrainzId { get; set; }
    public int TrackNumber { get; set; }
    public int DiscNumber { get; set; }
    public int? DurationSeconds { get; set; }
    public bool Explicit { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
    public AudioAnalysisResource? AudioAnalysis { get; set; }
}

public class BatchTracksResource
{
    public List<TrackWithAudioResource> Tracks { get; set; } = new();
}
