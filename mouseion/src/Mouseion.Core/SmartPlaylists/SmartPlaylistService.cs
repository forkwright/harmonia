// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Filtering;
using Mouseion.Core.Library;

namespace Mouseion.Core.SmartPlaylists;

public partial class SmartPlaylistService : ISmartPlaylistService
{
    private readonly ISmartPlaylistRepository _repository;
    private readonly ILibraryFilterService _filterService;
    private readonly ILogger<SmartPlaylistService> _logger;

    public SmartPlaylistService(
        ISmartPlaylistRepository repository,
        ILibraryFilterService filterService,
        ILogger<SmartPlaylistService> logger)
    {
        _repository = repository;
        _filterService = filterService;
        _logger = logger;
    }

    public async Task<IEnumerable<SmartPlaylist>> GetAllAsync(CancellationToken ct = default)
    {
        return await _repository.AllAsync(ct).ConfigureAwait(false);
    }

    public async Task<SmartPlaylist?> GetAsync(int id, CancellationToken ct = default)
    {
        return await _repository.FindAsync(id, ct).ConfigureAwait(false);
    }

    public async Task<SmartPlaylist> CreateAsync(SmartPlaylist playlist, CancellationToken ct = default)
    {
        var now = DateTime.UtcNow;
        playlist.CreatedAt = now;
        playlist.UpdatedAt = now;
        playlist.LastRefreshed = DateTime.MinValue;
        playlist.TrackCount = 0;

        var created = await _repository.InsertAsync(playlist, ct).ConfigureAwait(false);
        LogPlaylistCreated(created.Id, created.Name);
        return created;
    }

    public async Task<SmartPlaylist> UpdateAsync(SmartPlaylist playlist, CancellationToken ct = default)
    {
        playlist.UpdatedAt = DateTime.UtcNow;
        var updated = await _repository.UpdateAsync(playlist, ct).ConfigureAwait(false);
        LogPlaylistUpdated(updated.Id, updated.Name);
        return updated;
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(id, ct).ConfigureAwait(false);
        LogPlaylistDeleted(id);
    }

    public async Task<SmartPlaylist> RefreshAsync(int id, CancellationToken ct = default)
    {
        var playlist = await _repository.FindAsync(id, ct).ConfigureAwait(false)
            ?? throw new KeyNotFoundException($"SmartPlaylist with ID {id} not found");

        var filterRequest = JsonSerializer.Deserialize<FilterRequest>(playlist.FilterRequestJson)
            ?? new FilterRequest();

        // Remove page size limit for playlist population — get all matching tracks
        filterRequest.PageSize = int.MaxValue;

        var filterResult = await _filterService.FilterTracksAsync(filterRequest, ct).ConfigureAwait(false);

        var tracks = filterResult.Tracks
            .Select((t, i) => new SmartPlaylistTrack { TrackId = t.Id, Position = i })
            .ToList();

        await _repository.SetTracksAsync(id, tracks, ct).ConfigureAwait(false);

        playlist.TrackCount = tracks.Count;
        playlist.LastRefreshed = DateTime.UtcNow;
        playlist.UpdatedAt = DateTime.UtcNow;
        await _repository.UpdateAsync(playlist, ct).ConfigureAwait(false);

        LogPlaylistRefreshed(id, playlist.Name, tracks.Count);
        return playlist;
    }

    public async Task<IEnumerable<SmartPlaylistTrack>> GetTracksAsync(int id, CancellationToken ct = default)
    {
        return await _repository.GetTracksAsync(id, ct).ConfigureAwait(false);
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart playlist created: {Id} ({Name})")]
    private partial void LogPlaylistCreated(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart playlist updated: {Id} ({Name})")]
    private partial void LogPlaylistUpdated(int id, string name);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart playlist deleted: {Id}")]
    private partial void LogPlaylistDeleted(int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Smart playlist refreshed: {Id} ({Name}) — {TrackCount} tracks")]
    private partial void LogPlaylistRefreshed(int id, string name, int trackCount);
}
