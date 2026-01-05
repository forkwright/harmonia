// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Music;

public interface ITrackSearchService
{
    Task<List<TrackSearchResult>> SearchAsync(string query, int limit, CancellationToken ct = default);
    List<TrackSearchResult> Search(string query, int limit);
}

public class TrackSearchService : ITrackSearchService
{
    private readonly IDatabase _database;
    private readonly IMusicFileRepository _musicFileRepository;

    public TrackSearchService(
        IDatabase database,
        IMusicFileRepository musicFileRepository)
    {
        _database = database;
        _musicFileRepository = musicFileRepository;
    }

    public async Task<List<TrackSearchResult>> SearchAsync(string query, int limit, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(query))
        {
            return new List<TrackSearchResult>();
        }

        var normalizedQuery = query.Trim().ToLowerInvariant();
        var queryWords = normalizedQuery.Split(' ', StringSplitOptions.RemoveEmptyEntries);

        using var conn = _database.OpenConnection();

        // Search across tracks with joins to get artist/album names and genres
        var sql = @"
            SELECT
                m.*,
                art.Name as ArtistName,
                alb.Title as AlbumTitle,
                alb.Genres as AlbumGenres
            FROM ""MediaItems"" m
            LEFT JOIN ""Artists"" art ON m.""ArtistId"" = art.""Id""
            LEFT JOIN ""Albums"" alb ON m.""AlbumId"" = alb.""Id""
            WHERE m.""MediaType"" = @MediaType
            ORDER BY m.""Id""
        ";

        var trackData = await conn.QueryAsync<dynamic>(
            sql,
            new { MediaType = (int)MediaType.Music }).ConfigureAwait(false);

        var results = new List<TrackSearchResult>();

        foreach (var row in trackData)
        {
            var track = MapToTrack(row);
            var artistName = row.ArtistName is string a ? a : null;
            var albumTitle = row.AlbumTitle is string t ? t : null;
            var albumGenres = row.AlbumGenres is string g ? g : null;

            // Parse genres (stored as JSON array string)
            var genres = ParseGenres(albumGenres);
            var primaryGenre = genres.FirstOrDefault();

            // Calculate relevance score
            var score = CalculateRelevanceScore(
                track.Title,
                artistName,
                albumTitle,
                genres,
                normalizedQuery,
                queryWords);

            if (score > 0)
            {
                // Get audio quality info from MusicFile
                int? bitDepth = null;
                int? dynamicRange = null;
                bool lossless = false;

                var musicFiles = await _musicFileRepository.GetByTrackIdAsync(track.Id, ct).ConfigureAwait(false);
                if (musicFiles.Count > 0)
                {
                    var musicFile = musicFiles[0];
                    // For now, we'll extract from the audio file path or metadata
                    // This will be enhanced when we add explicit columns to MusicFiles table
                    bitDepth = ExtractBitDepth(musicFile.AudioFormat);
                    lossless = IsLosslessFormat(musicFile.AudioFormat);
                }

                results.Add(new TrackSearchResult
                {
                    Track = track,
                    ArtistName = artistName,
                    AlbumTitle = albumTitle,
                    Genre = primaryGenre,
                    BitDepth = bitDepth,
                    DynamicRange = dynamicRange,
                    Lossless = lossless,
                    RelevanceScore = score
                });
            }
        }

        return results
            .OrderByDescending(r => r.RelevanceScore)
            .Take(limit)
            .ToList();
    }

    public List<TrackSearchResult> Search(string query, int limit)
    {
        return SearchAsync(query, limit).GetAwaiter().GetResult();
    }

    private static Track MapToTrack(dynamic row)
    {
        return new Track
        {
            Id = row.Id,
            AlbumId = row.AlbumId,
            ArtistId = row.ArtistId,
            Title = row.Title,
            ForeignTrackId = row.ForeignTrackId,
            MusicBrainzId = row.MusicBrainzId,
            TrackNumber = row.TrackNumber,
            DiscNumber = row.DiscNumber,
            DurationSeconds = row.DurationSeconds,
            Explicit = row.Explicit,
            Monitored = row.Monitored,
            QualityProfileId = row.QualityProfileId,
            Path = row.Path,
            RootFolderPath = row.RootFolderPath,
            Added = row.Added
        };
    }

    private static List<string> ParseGenres(string? genresJson)
    {
        if (string.IsNullOrWhiteSpace(genresJson))
        {
            return new List<string>();
        }

        try
        {
            // Simple JSON array parsing: ["Genre1","Genre2"]
            var cleaned = genresJson.Trim('[', ']');
            if (string.IsNullOrWhiteSpace(cleaned))
            {
                return new List<string>();
            }

            return cleaned
                .Split(',')
                .Select(g => g.Trim(' ', '"'))
                .Where(g => !string.IsNullOrWhiteSpace(g))
                .ToList();
        }
        catch
        {
            return new List<string>();
        }
    }

    private static double CalculateRelevanceScore(
        string trackTitle,
        string? artistName,
        string? albumTitle,
        List<string> genres,
        string normalizedQuery,
        string[] queryWords)
    {
        var baseScore = GetBaseMatchScore(trackTitle, artistName, albumTitle, genres, normalizedQuery);
        var multiWordBonus = GetMultiWordBonus(trackTitle, artistName, albumTitle, genres, queryWords);
        return baseScore + multiWordBonus;
    }

    private static double GetBaseMatchScore(
        string trackTitle,
        string? artistName,
        string? albumTitle,
        List<string> genres,
        string normalizedQuery)
    {
        // Exact match (highest priority)
        if (trackTitle?.Equals(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 100.0;
        if (artistName?.Equals(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 90.0;
        if (albumTitle?.Equals(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 85.0;
        if (genres.Any(g => g.Equals(normalizedQuery, StringComparison.OrdinalIgnoreCase))) return 80.0;

        // Starts with (second priority)
        if (trackTitle?.StartsWith(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 70.0;
        if (artistName?.StartsWith(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 65.0;
        if (albumTitle?.StartsWith(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 60.0;

        // Contains (third priority)
        if (trackTitle?.Contains(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 50.0;
        if (artistName?.Contains(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 45.0;
        if (albumTitle?.Contains(normalizedQuery, StringComparison.OrdinalIgnoreCase) ?? false) return 40.0;
        if (genres.Any(g => g.Contains(normalizedQuery, StringComparison.OrdinalIgnoreCase))) return 35.0;

        return 0.0;
    }

    private static double GetMultiWordBonus(
        string trackTitle,
        string? artistName,
        string? albumTitle,
        List<string> genres,
        string[] queryWords)
    {
        if (queryWords.Length <= 1)
            return 0.0;

        var trackLower = trackTitle?.ToLowerInvariant() ?? "";
        var artistLower = artistName?.ToLowerInvariant() ?? "";
        var albumLower = albumTitle?.ToLowerInvariant() ?? "";
        var genresLower = string.Join(" ", genres).ToLowerInvariant();

        if (queryWords.All(w => trackLower.Contains(w))) return 30.0;
        if (queryWords.All(w => artistLower.Contains(w))) return 25.0;
        if (queryWords.All(w => albumLower.Contains(w))) return 20.0;
        if (queryWords.All(w => genresLower.Contains(w))) return 15.0;

        return 0.0;
    }

    private static int? ExtractBitDepth(string? audioFormat)
    {
        if (string.IsNullOrWhiteSpace(audioFormat))
        {
            return null;
        }

        // Extract bit depth from format string (e.g., "FLAC 24-bit", "PCM 16-bit")
        if (audioFormat.Contains("24", StringComparison.OrdinalIgnoreCase))
        {
            return 24;
        }
        if (audioFormat.Contains("16", StringComparison.OrdinalIgnoreCase))
        {
            return 16;
        }
        if (audioFormat.Contains("32", StringComparison.OrdinalIgnoreCase))
        {
            return 32;
        }

        return null;
    }

    private static bool IsLosslessFormat(string? audioFormat)
    {
        if (string.IsNullOrWhiteSpace(audioFormat))
        {
            return false;
        }

        return audioFormat.Contains("FLAC", StringComparison.OrdinalIgnoreCase) ||
               audioFormat.Contains("PCM", StringComparison.OrdinalIgnoreCase) ||
               audioFormat.Contains("APE", StringComparison.OrdinalIgnoreCase) ||
               audioFormat.Contains("WavPack", StringComparison.OrdinalIgnoreCase) ||
               audioFormat.Contains("DSD", StringComparison.OrdinalIgnoreCase);
    }
}

public class TrackSearchResult
{
    public Track Track { get; set; } = null!;
    public string? ArtistName { get; set; }
    public string? AlbumTitle { get; set; }
    public string? Genre { get; set; }
    public int? BitDepth { get; set; }
    public int? DynamicRange { get; set; }
    public bool Lossless { get; set; }
    public double RelevanceScore { get; set; }
}
