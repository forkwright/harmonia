// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.Music;

public interface IAddArtistService
{
    Task<Artist> AddArtistAsync(Artist artist, CancellationToken ct = default);
    Task<List<Artist>> AddArtistsAsync(List<Artist> artists, CancellationToken ct = default);

    Artist AddArtist(Artist artist);
    List<Artist> AddArtists(List<Artist> artists);
}

public class AddArtistService : IAddArtistService
{
    private readonly IArtistRepository _artistRepository;
    private readonly ILogger<AddArtistService> _logger;

    public AddArtistService(
        IArtistRepository artistRepository,
        ILogger<AddArtistService> logger)
    {
        _artistRepository = artistRepository;
        _logger = logger;
    }

    public async Task<Artist> AddArtistAsync(Artist artist, CancellationToken ct = default)
    {
        ValidateArtist(artist);

        var existing = await _artistRepository.FindByNameAsync(artist.Name, ct).ConfigureAwait(false);
        if (existing != null)
        {
            _logger.LogInformation("Artist already exists: {ArtistName}", artist.Name.SanitizeForLog());
            return existing;
        }

        artist.Added = DateTime.UtcNow;
        artist.Monitored = true;

        var added = await _artistRepository.InsertAsync(artist, ct).ConfigureAwait(false);
        _logger.LogInformation("Added artist: {ArtistName} - MusicBrainz: {MusicBrainzId}",
            added.Name.SanitizeForLog(), added.MusicBrainzId?.SanitizeForLog());

        return added;
    }

    public Artist AddArtist(Artist artist)
    {
        ValidateArtist(artist);

        var existing = _artistRepository.FindByName(artist.Name);
        if (existing != null)
        {
            _logger.LogInformation("Artist already exists: {ArtistName}", artist.Name.SanitizeForLog());
            return existing;
        }

        artist.Added = DateTime.UtcNow;
        artist.Monitored = true;

        var added = _artistRepository.Insert(artist);
        _logger.LogInformation("Added artist: {ArtistName} - MusicBrainz: {MusicBrainzId}",
            added.Name.SanitizeForLog(), added.MusicBrainzId?.SanitizeForLog());

        return added;
    }

    public async Task<List<Artist>> AddArtistsAsync(List<Artist> artists, CancellationToken ct = default)
    {
        var addedArtists = new List<Artist>();

        foreach (var artist in artists)
        {
            try
            {
                var added = await AddArtistAsync(artist, ct).ConfigureAwait(false);
                addedArtists.Add(added);
            }
            catch (ArgumentException ex)
            {
                _logger.LogError(ex, "Validation failed for artist: {ArtistName}", artist.Name.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Error adding artist: {ArtistName}", artist.Name.SanitizeForLog());
            }
        }

        return addedArtists;
    }

    public List<Artist> AddArtists(List<Artist> artists)
    {
        var addedArtists = new List<Artist>();

        foreach (var artist in artists)
        {
            try
            {
                var added = AddArtist(artist);
                addedArtists.Add(added);
            }
            catch (ArgumentException ex)
            {
                _logger.LogError(ex, "Validation failed for artist: {ArtistName}", artist.Name.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                _logger.LogError(ex, "Error adding artist: {ArtistName}", artist.Name.SanitizeForLog());
            }
        }

        return addedArtists;
    }

    private void ValidateArtist(Artist artist)
    {
        if (string.IsNullOrWhiteSpace(artist.Name))
        {
            throw new ArgumentException("Artist name is required", nameof(artist));
        }

        if (artist.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(artist));
        }
    }
}
