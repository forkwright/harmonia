// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Download.Strm;

/// <summary>
/// Debrid service configuration: Real-Debrid, AllDebrid, Premiumize.
/// Resolves torrent/magnet links to direct HTTPS stream URLs.
/// </summary>
public class DebridServiceDefinition : ModelBase
{
    public string Name { get; set; } = string.Empty;
    public DebridProvider Provider { get; set; }
    public string ApiKey { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public int Priority { get; set; }
    public int? BandwidthLimitGb { get; set; }
    public decimal? BandwidthUsedGb { get; set; }
    public DateTime? BandwidthResetDate { get; set; }
    public DateTime? LastChecked { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

public enum DebridProvider
{
    RealDebrid = 0,
    AllDebrid = 1,
    Premiumize = 2
}

/// <summary>
/// Tracking record for a .strm file: which media item, where it points, validity.
/// </summary>
public class StrmFile : ModelBase
{
    public int MediaItemId { get; set; }
    public int? DebridServiceId { get; set; }
    public string FilePath { get; set; } = string.Empty;
    public string StreamUrl { get; set; } = string.Empty;
    public string? Quality { get; set; }
    public long? SizeBytes { get; set; }
    public bool IsValid { get; set; } = true;
    public DateTime? LastVerified { get; set; }
    public DateTime? ExpiresAt { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

/// <summary>
/// Result of resolving a magnet/torrent through a debrid service.
/// </summary>
public class DebridResolveResult
{
    public bool Success { get; set; }
    public string? StreamUrl { get; set; }
    public string? FileName { get; set; }
    public long? SizeBytes { get; set; }
    public string? Quality { get; set; }
    public DateTime? ExpiresAt { get; set; }
    public string? ErrorMessage { get; set; }

    public static DebridResolveResult Failure(string error) => new() { Success = false, ErrorMessage = error };
    public static DebridResolveResult Ok(string url, string? fileName = null, long? size = null, DateTime? expires = null) =>
        new() { Success = true, StreamUrl = url, FileName = fileName, SizeBytes = size, ExpiresAt = expires };
}
