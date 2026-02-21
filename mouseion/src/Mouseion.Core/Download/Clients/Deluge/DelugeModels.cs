// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Download.Clients.Deluge;

/// <summary>
/// Deluge Web UI JSON-RPC models.
/// Deluge uses its own JSON-RPC variant with auth via web.login method.
/// </summary>
public class DelugeJsonRpcRequest
{
    [JsonPropertyName("method")]
    public string Method { get; set; } = string.Empty;

    [JsonPropertyName("params")]
    public object[] Params { get; set; } = Array.Empty<object>();

    [JsonPropertyName("id")]
    public int Id { get; set; } = 1;
}

public class DelugeJsonRpcResponse<T>
{
    [JsonPropertyName("result")]
    public T? Result { get; set; }

    [JsonPropertyName("error")]
    public DelugeError? Error { get; set; }

    [JsonPropertyName("id")]
    public int Id { get; set; }
}

public class DelugeError
{
    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;

    [JsonPropertyName("code")]
    public int Code { get; set; }
}

/// <summary>
/// Result of core.get_torrents_status call — maps hash to torrent fields
/// </summary>
public class DelugeTorrent
{
    [JsonPropertyName("hash")]
    public string Hash { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("state")]
    public string State { get; set; } = string.Empty;

    [JsonPropertyName("total_size")]
    public long TotalSize { get; set; }

    [JsonPropertyName("progress")]
    public double Progress { get; set; }

    [JsonPropertyName("eta")]
    public long Eta { get; set; }

    [JsonPropertyName("ratio")]
    public double Ratio { get; set; }

    [JsonPropertyName("save_path")]
    public string SavePath { get; set; } = string.Empty;

    [JsonPropertyName("label")]
    public string Label { get; set; } = string.Empty;

    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;

    [JsonPropertyName("is_finished")]
    public bool IsFinished { get; set; }

    [JsonPropertyName("total_remaining")]
    public long TotalRemaining { get; set; }

    [JsonPropertyName("paused")]
    public bool Paused { get; set; }

    [JsonPropertyName("tracker_status")]
    public string TrackerStatus { get; set; } = string.Empty;
}
