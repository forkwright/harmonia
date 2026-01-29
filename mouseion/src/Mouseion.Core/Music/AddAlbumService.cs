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

public interface IAddAlbumService
{
    Task<Album> AddAlbumAsync(Album album, CancellationToken ct = default);
    Task<List<Album>> AddAlbumsAsync(List<Album> albums, CancellationToken ct = default);

    Album AddAlbum(Album album);
    List<Album> AddAlbums(List<Album> albums);
}

public partial class AddAlbumService : IAddAlbumService
{
    private readonly IAlbumRepository _albumRepository;
    private readonly IArtistRepository _artistRepository;
    private readonly ILogger<AddAlbumService> _logger;

    public AddAlbumService(
        IAlbumRepository albumRepository,
        IArtistRepository artistRepository,
        ILogger<AddAlbumService> logger)
    {
        _albumRepository = albumRepository;
        _artistRepository = artistRepository;
        _logger = logger;
    }

    public async Task<Album> AddAlbumAsync(Album album, CancellationToken ct = default)
    {
        ValidateAlbum(album);

        if (album.ArtistId.HasValue)
        {
            var artist = await _artistRepository.FindAsync(album.ArtistId.Value, ct).ConfigureAwait(false);
            if (artist == null)
            {
                throw new ArgumentException($"Artist with ID {album.ArtistId.Value} not found", nameof(album));
            }
        }

        var existing = await _albumRepository.FindByTitleAsync(album.Title, album.ArtistId, ct).ConfigureAwait(false);
        if (existing != null)
        {
            LogAlbumAlreadyExists(album.Title.SanitizeForLog(), album.ArtistId);
            return existing;
        }

        album.Added = DateTime.UtcNow;
        album.Monitored = true;

        var added = await _albumRepository.InsertAsync(album, ct).ConfigureAwait(false);
        LogAlbumAdded(added.Title.SanitizeForLog(), added.ReleaseDate?.Year, added.MusicBrainzId?.SanitizeForLog(), added.ArtistId);

        return added;
    }

    public Album AddAlbum(Album album)
    {
        ValidateAlbum(album);

        if (album.ArtistId.HasValue)
        {
            var artist = _artistRepository.Find(album.ArtistId.Value);
            if (artist == null)
            {
                throw new ArgumentException($"Artist with ID {album.ArtistId.Value} not found", nameof(album));
            }
        }

        var existing = _albumRepository.FindByTitle(album.Title, album.ArtistId);
        if (existing != null)
        {
            LogAlbumAlreadyExists(album.Title.SanitizeForLog(), album.ArtistId);
            return existing;
        }

        album.Added = DateTime.UtcNow;
        album.Monitored = true;

        var added = _albumRepository.Insert(album);
        LogAlbumAdded(added.Title.SanitizeForLog(), added.ReleaseDate?.Year, added.MusicBrainzId?.SanitizeForLog(), added.ArtistId);

        return added;
    }

    public async Task<List<Album>> AddAlbumsAsync(List<Album> albums, CancellationToken ct = default)
    {
        var addedAlbums = new List<Album>();

        foreach (var album in albums)
        {
            try
            {
                var added = await AddAlbumAsync(album, ct).ConfigureAwait(false);
                addedAlbums.Add(added);
            }
            catch (ArgumentException ex)
            {
                LogValidationFailed(ex, album.Title.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                LogAddingError(ex, album.Title.SanitizeForLog());
            }
        }

        return addedAlbums;
    }

    public List<Album> AddAlbums(List<Album> albums)
    {
        var addedAlbums = new List<Album>();

        foreach (var album in albums)
        {
            try
            {
                var added = AddAlbum(album);
                addedAlbums.Add(added);
            }
            catch (ArgumentException ex)
            {
                LogValidationFailed(ex, album.Title.SanitizeForLog());
            }
            catch (InvalidOperationException ex)
            {
                LogAddingError(ex, album.Title.SanitizeForLog());
            }
        }

        return addedAlbums;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Album already exists: {AlbumTitle} by Artist {ArtistId}")]
    private partial void LogAlbumAlreadyExists(string albumTitle, int? artistId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added album: {AlbumTitle} ({ReleaseDate}) - MusicBrainz: {MusicBrainzId}, Artist ID: {ArtistId}")]
    private partial void LogAlbumAdded(string albumTitle, int? releaseDate, string? musicBrainzId, int? artistId);

    [LoggerMessage(Level = LogLevel.Error, Message = "Validation failed for album: {AlbumTitle}")]
    private partial void LogValidationFailed(Exception ex, string albumTitle);

    [LoggerMessage(Level = LogLevel.Error, Message = "Error adding album: {AlbumTitle}")]
    private partial void LogAddingError(Exception ex, string albumTitle);

    private void ValidateAlbum(Album album)
    {
        if (string.IsNullOrWhiteSpace(album.Title))
        {
            throw new ArgumentException("Album title is required", nameof(album));
        }

        if (album.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(album));
        }
    }
}
