// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.ImportLists.LastFm;

// Last.fm API response models
// Docs: https://www.last.fm/api

public class LastFmResponse<T>
{
    [JsonPropertyName("error")]
    public int? Error { get; set; }

    [JsonPropertyName("message")]
    public string? Message { get; set; }
}

// user.getTopArtists
public class LastFmTopArtistsResponse
{
    [JsonPropertyName("topartists")]
    public LastFmTopArtistsContainer TopArtists { get; set; } = new();
}

public class LastFmTopArtistsContainer
{
    [JsonPropertyName("artist")]
    public List<LastFmArtist> Artists { get; set; } = new();

    [JsonPropertyName("@attr")]
    public LastFmPaginationAttr Attr { get; set; } = new();
}

public class LastFmArtist
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("playcount")]
    public string PlayCount { get; set; } = "0";

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }

    [JsonPropertyName("url")]
    public string? Url { get; set; }
}

// user.getTopAlbums
public class LastFmTopAlbumsResponse
{
    [JsonPropertyName("topalbums")]
    public LastFmTopAlbumsContainer TopAlbums { get; set; } = new();
}

public class LastFmTopAlbumsContainer
{
    [JsonPropertyName("album")]
    public List<LastFmAlbum> Albums { get; set; } = new();

    [JsonPropertyName("@attr")]
    public LastFmPaginationAttr Attr { get; set; } = new();
}

public class LastFmAlbum
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("playcount")]
    public string PlayCount { get; set; } = "0";

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }

    [JsonPropertyName("artist")]
    public LastFmArtist Artist { get; set; } = new();

    [JsonPropertyName("url")]
    public string? Url { get; set; }
}

// user.getRecentTracks
public class LastFmRecentTracksResponse
{
    [JsonPropertyName("recenttracks")]
    public LastFmRecentTracksContainer RecentTracks { get; set; } = new();
}

public class LastFmRecentTracksContainer
{
    [JsonPropertyName("track")]
    public List<LastFmTrack> Tracks { get; set; } = new();

    [JsonPropertyName("@attr")]
    public LastFmPaginationAttr Attr { get; set; } = new();
}

public class LastFmTrack
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("artist")]
    public LastFmTrackArtist Artist { get; set; } = new();

    [JsonPropertyName("album")]
    public LastFmTrackAlbum Album { get; set; } = new();

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }

    [JsonPropertyName("date")]
    public LastFmDate? Date { get; set; }

    [JsonPropertyName("@attr")]
    public LastFmTrackAttr? Attr { get; set; }

    /// <summary>
    /// True if this is the currently playing track (has @attr.nowplaying="true").
    /// </summary>
    public bool IsNowPlaying => Attr?.NowPlaying == "true";
}

public class LastFmTrackArtist
{
    [JsonPropertyName("#text")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }
}

public class LastFmTrackAlbum
{
    [JsonPropertyName("#text")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }
}

public class LastFmDate
{
    /// <summary>
    /// Unix timestamp of the scrobble.
    /// </summary>
    [JsonPropertyName("uts")]
    public string Uts { get; set; } = "0";

    [JsonPropertyName("#text")]
    public string Text { get; set; } = string.Empty;
}

public class LastFmTrackAttr
{
    [JsonPropertyName("nowplaying")]
    public string? NowPlaying { get; set; }
}

// user.getLovedTracks
public class LastFmLovedTracksResponse
{
    [JsonPropertyName("lovedtracks")]
    public LastFmLovedTracksContainer LovedTracks { get; set; } = new();
}

public class LastFmLovedTracksContainer
{
    [JsonPropertyName("track")]
    public List<LastFmLovedTrack> Tracks { get; set; } = new();

    [JsonPropertyName("@attr")]
    public LastFmPaginationAttr Attr { get; set; } = new();
}

public class LastFmLovedTrack
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("artist")]
    public LastFmTrackArtist Artist { get; set; } = new();

    [JsonPropertyName("mbid")]
    public string? Mbid { get; set; }

    [JsonPropertyName("date")]
    public LastFmDate? Date { get; set; }
}

// Shared pagination
public class LastFmPaginationAttr
{
    [JsonPropertyName("page")]
    public string Page { get; set; } = "1";

    [JsonPropertyName("perPage")]
    public string PerPage { get; set; } = "50";

    [JsonPropertyName("totalPages")]
    public string TotalPages { get; set; } = "1";

    [JsonPropertyName("total")]
    public string Total { get; set; } = "0";
}
