// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Download.Clients.Transmission;

/// <summary>
/// Transmission RPC request envelope
/// </summary>
public class TransmissionRequest
{
    [JsonPropertyName("method")]
    public string Method { get; set; } = string.Empty;

    [JsonPropertyName("arguments")]
    public object? Arguments { get; set; }

    [JsonPropertyName("tag")]
    public int? Tag { get; set; }
}

/// <summary>
/// Transmission RPC response envelope
/// </summary>
public class TransmissionResponse<T>
{
    [JsonPropertyName("result")]
    public string Result { get; set; } = string.Empty;

    [JsonPropertyName("arguments")]
    public T? Arguments { get; set; }

    [JsonPropertyName("tag")]
    public int? Tag { get; set; }

    public bool IsSuccess => Result == "success";
}

public class TransmissionTorrentList
{
    [JsonPropertyName("torrents")]
    public List<TransmissionTorrent> Torrents { get; set; } = new();
}

public class TransmissionTorrent
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("hashString")]
    public string HashString { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("status")]
    public int Status { get; set; }

    [JsonPropertyName("totalSize")]
    public long TotalSize { get; set; }

    [JsonPropertyName("percentDone")]
    public double PercentDone { get; set; }

    [JsonPropertyName("eta")]
    public long Eta { get; set; }

    [JsonPropertyName("uploadRatio")]
    public double UploadRatio { get; set; }

    [JsonPropertyName("downloadDir")]
    public string DownloadDir { get; set; } = string.Empty;

    [JsonPropertyName("labels")]
    public List<string> Labels { get; set; } = new();

    [JsonPropertyName("errorString")]
    public string ErrorString { get; set; } = string.Empty;

    [JsonPropertyName("error")]
    public int Error { get; set; }

    [JsonPropertyName("isFinished")]
    public bool IsFinished { get; set; }

    [JsonPropertyName("leftUntilDone")]
    public long LeftUntilDone { get; set; }
}

public class TransmissionSessionInfo
{
    [JsonPropertyName("download-dir")]
    public string DownloadDir { get; set; } = string.Empty;

    [JsonPropertyName("version")]
    public string Version { get; set; } = string.Empty;

    [JsonPropertyName("rpc-version")]
    public int RpcVersion { get; set; }
}

/// <summary>
/// Transmission torrent status codes (v2.94+)
/// 0 = stopped, 1 = check pending, 2 = checking, 3 = download pending,
/// 4 = downloading, 5 = seed pending, 6 = seeding
/// </summary>
public static class TransmissionTorrentStatus
{
    public const int Stopped = 0;
    public const int CheckPending = 1;
    public const int Checking = 2;
    public const int DownloadPending = 3;
    public const int Downloading = 4;
    public const int SeedPending = 5;
    public const int Seeding = 6;
}
