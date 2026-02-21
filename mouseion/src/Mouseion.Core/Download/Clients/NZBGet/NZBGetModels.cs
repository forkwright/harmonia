// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Download.Clients.NZBGet;

/// <summary>
/// NZBGet uses JSON-RPC, so requests/responses follow that pattern.
/// </summary>
public class JsonRpcRequest
{
    [JsonPropertyName("method")]
    public string Method { get; set; } = string.Empty;

    [JsonPropertyName("params")]
    public object[] Params { get; set; } = Array.Empty<object>();

    [JsonPropertyName("id")]
    public int Id { get; set; } = 1;

    [JsonPropertyName("jsonrpc")]
    public string JsonRpc { get; set; } = "2.0";
}

public class JsonRpcResponse<T>
{
    [JsonPropertyName("result")]
    public T Result { get; set; } = default!;

    [JsonPropertyName("error")]
    public JsonRpcError? Error { get; set; }
}

public class JsonRpcError
{
    [JsonPropertyName("code")]
    public int Code { get; set; }

    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}

public class NZBGetQueueItem
{
    [JsonPropertyName("NZBID")]
    public int NzbId { get; set; }

    [JsonPropertyName("NZBName")]
    public string NzbName { get; set; } = string.Empty;

    [JsonPropertyName("Category")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("FileSizeLo")]
    public long FileSizeLo { get; set; }

    [JsonPropertyName("FileSizeHi")]
    public long FileSizeHi { get; set; }

    [JsonPropertyName("RemainingSizeLo")]
    public long RemainingSizeLo { get; set; }

    [JsonPropertyName("RemainingSizeHi")]
    public long RemainingSizeHi { get; set; }

    [JsonPropertyName("Status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("DestDir")]
    public string DestDir { get; set; } = string.Empty;

    public long FileSize => (FileSizeHi << 32) + FileSizeLo;
    public long RemainingSize => (RemainingSizeHi << 32) + RemainingSizeLo;
}

public class NZBGetHistoryItem
{
    [JsonPropertyName("NZBID")]
    public int NzbId { get; set; }

    [JsonPropertyName("Name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("Category")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("FileSizeLo")]
    public long FileSizeLo { get; set; }

    [JsonPropertyName("FileSizeHi")]
    public long FileSizeHi { get; set; }

    [JsonPropertyName("Status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("DestDir")]
    public string DestDir { get; set; } = string.Empty;

    [JsonPropertyName("ParStatus")]
    public string ParStatus { get; set; } = string.Empty;

    [JsonPropertyName("UnpackStatus")]
    public string UnpackStatus { get; set; } = string.Empty;

    public long FileSize => (FileSizeHi << 32) + FileSizeLo;
}

public class NZBGetStatus
{
    [JsonPropertyName("ServerStandBy")]
    public bool ServerStandBy { get; set; }

    [JsonPropertyName("DownloadRate")]
    public long DownloadRate { get; set; }

    [JsonPropertyName("DownloadPaused")]
    public bool DownloadPaused { get; set; }

    [JsonPropertyName("FreeDiskSpaceMB")]
    public long FreeDiskSpaceMB { get; set; }
}

public class NZBGetVersionResponse
{
    [JsonPropertyName("result")]
    public string Version { get; set; } = string.Empty;
}
