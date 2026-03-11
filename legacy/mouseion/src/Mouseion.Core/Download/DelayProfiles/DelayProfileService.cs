// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Download.DelayProfiles;

public interface IDelayProfileService
{
    Task<IEnumerable<DelayProfile>> GetAllAsync(CancellationToken ct = default);
    Task<DelayProfile?> GetAsync(int id, CancellationToken ct = default);
    Task<DelayProfile> CreateAsync(DelayProfile profile, CancellationToken ct = default);
    Task<DelayProfile> UpdateAsync(DelayProfile profile, CancellationToken ct = default);
    Task DeleteAsync(int id, CancellationToken ct = default);

    /// <summary>
    /// Evaluate whether a release should be delayed or grabbed immediately.
    /// Returns the delay in hours (0 = grab now).
    /// </summary>
    Task<DelayEvaluation> EvaluateAsync(
        MediaType mediaType,
        DownloadProtocol protocol,
        int qualityWeight,
        bool hasPreferredWords,
        IEnumerable<int> tagIds,
        CancellationToken ct = default);
}

/// <summary>
/// Result of delay profile evaluation.
/// </summary>
public class DelayEvaluation
{
    /// <summary>Hours to delay. 0 = grab immediately.</summary>
    public int DelayHours { get; set; }

    /// <summary>Whether the release bypassed delay due to quality.</summary>
    public bool BypassedByQuality { get; set; }

    /// <summary>Whether the release bypassed delay due to preferred words.</summary>
    public bool BypassedByPreferredWords { get; set; }

    /// <summary>The profile that was matched, if any.</summary>
    public int? MatchedProfileId { get; set; }

    /// <summary>Human-readable reason for the decision.</summary>
    public string Reason { get; set; } = string.Empty;

    public static DelayEvaluation GrabNow(string reason = "No matching delay profile") => new()
    {
        DelayHours = 0,
        Reason = reason
    };
}

public partial class DelayProfileService : IDelayProfileService
{
    private readonly IDelayProfileRepository _repository;
    private readonly ILogger<DelayProfileService> _logger;

    public DelayProfileService(IDelayProfileRepository repository, ILogger<DelayProfileService> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    public async Task<IEnumerable<DelayProfile>> GetAllAsync(CancellationToken ct = default)
        => await _repository.AllAsync(ct).ConfigureAwait(false);

    public async Task<DelayProfile?> GetAsync(int id, CancellationToken ct = default)
        => await _repository.FindAsync(id, ct).ConfigureAwait(false);

    public async Task<DelayProfile> CreateAsync(DelayProfile profile, CancellationToken ct = default)
    {
        var created = await _repository.InsertAsync(profile, ct).ConfigureAwait(false);
        LogCreated(created.Id, created.Name);
        return created;
    }

    public async Task<DelayProfile> UpdateAsync(DelayProfile profile, CancellationToken ct = default)
    {
        var updated = await _repository.UpdateAsync(profile, ct).ConfigureAwait(false);
        LogUpdated(updated.Id, updated.Name);
        return updated;
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
        LogDeleted(id);
    }

    public async Task<DelayEvaluation> EvaluateAsync(
        MediaType mediaType,
        DownloadProtocol protocol,
        int qualityWeight,
        bool hasPreferredWords,
        IEnumerable<int> tagIds,
        CancellationToken ct = default)
    {
        var profile = await _repository.GetBestMatchAsync(mediaType, tagIds, ct).ConfigureAwait(false);
        if (profile == null)
        {
            return DelayEvaluation.GrabNow();
        }

        // Check quality bypass
        if (profile.BypassIfPreferredQuality && qualityWeight >= profile.PreferredQualityWeight)
        {
            LogBypassQuality(profile.Id, qualityWeight, profile.PreferredQualityWeight);
            return new DelayEvaluation
            {
                DelayHours = 0,
                BypassedByQuality = true,
                MatchedProfileId = profile.Id,
                Reason = $"Quality weight {qualityWeight} meets preferred threshold {profile.PreferredQualityWeight}"
            };
        }

        // Check preferred words bypass
        if (profile.BypassIfPreferredWords && hasPreferredWords)
        {
            LogBypassWords(profile.Id);
            return new DelayEvaluation
            {
                DelayHours = 0,
                BypassedByPreferredWords = true,
                MatchedProfileId = profile.Id,
                Reason = "Release has preferred words, bypassing delay"
            };
        }

        // Apply protocol-specific delay
        var delayHours = protocol switch
        {
            DownloadProtocol.Usenet => profile.UsenetDelay,
            DownloadProtocol.Torrent => profile.TorrentDelay,
            _ => 0
        };

        if (delayHours > 0)
        {
            LogDelayed(profile.Id, delayHours, protocol.ToString());
        }

        return new DelayEvaluation
        {
            DelayHours = delayHours,
            MatchedProfileId = profile.Id,
            Reason = delayHours > 0
                ? $"Delaying {delayHours}h for {protocol} — quality {qualityWeight} below preferred {profile.PreferredQualityWeight}"
                : $"No delay configured for {protocol} on profile {profile.Id}"
        };
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Delay profile created: {Id} ({Name})")]
    private partial void LogCreated(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Delay profile updated: {Id} ({Name})")]
    private partial void LogUpdated(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Delay profile deleted: {Id}")]
    private partial void LogDeleted(int id);

    [LoggerMessage(Level = LogLevel.Debug, Message = "Profile {ProfileId}: quality bypass — weight {Weight} >= preferred {Preferred}")]
    private partial void LogBypassQuality(int profileId, int weight, int preferred);

    [LoggerMessage(Level = LogLevel.Debug, Message = "Profile {ProfileId}: preferred words bypass")]
    private partial void LogBypassWords(int profileId);

    [LoggerMessage(Level = LogLevel.Debug, Message = "Profile {ProfileId}: delaying {Hours}h for {Protocol}")]
    private partial void LogDelayed(int profileId, int hours, string protocol);
}
