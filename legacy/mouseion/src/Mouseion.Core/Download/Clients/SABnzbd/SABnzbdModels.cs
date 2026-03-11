// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace Mouseion.Core.Download.Clients.SABnzbd;

public class SABnzbdQueue
{
    [JsonPropertyName("slots")]
    public List<SABnzbdQueueItem> Slots { get; set; } = new();

    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("speedlimit")]
    public string SpeedLimit { get; set; } = string.Empty;
}

public class SABnzbdHistory
{
    [JsonPropertyName("slots")]
    public List<SABnzbdHistoryItem> Slots { get; set; } = new();
}

public class SABnzbdQueueItem
{
    [JsonPropertyName("nzo_id")]
    public string NzoId { get; set; } = string.Empty;

    [JsonPropertyName("filename")]
    public string FileName { get; set; } = string.Empty;

    [JsonPropertyName("cat")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("mb")]
    public string TotalMb { get; set; } = "0";

    [JsonPropertyName("mbleft")]
    public string RemainingMb { get; set; } = "0";

    [JsonPropertyName("timeleft")]
    public string TimeLeft { get; set; } = string.Empty;

    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("priority")]
    public string Priority { get; set; } = string.Empty;

    [JsonPropertyName("percentage")]
    public string Percentage { get; set; } = "0";
}

public class SABnzbdHistoryItem
{
    [JsonPropertyName("nzo_id")]
    public string NzoId { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("category")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("bytes")]
    public long Bytes { get; set; }

    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;

    [JsonPropertyName("storage")]
    public string Storage { get; set; } = string.Empty;

    [JsonPropertyName("fail_message")]
    public string FailMessage { get; set; } = string.Empty;

    [JsonPropertyName("stage_log")]
    public List<SABnzbdStageLog>? StageLog { get; set; }
}

public class SABnzbdStageLog
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("actions")]
    public List<string> Actions { get; set; } = new();
}

public class SABnzbdConfig
{
    [JsonPropertyName("misc")]
    public SABnzbdMisc? Misc { get; set; }

    [JsonPropertyName("categories")]
    public List<SABnzbdCategory>? Categories { get; set; }
}

public class SABnzbdMisc
{
    [JsonPropertyName("complete_dir")]
    public string CompleteDir { get; set; } = string.Empty;

    [JsonPropertyName("download_dir")]
    public string DownloadDir { get; set; } = string.Empty;
}

public class SABnzbdCategory
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("dir")]
    public string Dir { get; set; } = string.Empty;
}

public class SABnzbdFullStatus
{
    [JsonPropertyName("version")]
    public string Version { get; set; } = string.Empty;
}

public class SABnzbdQueueResponse
{
    [JsonPropertyName("queue")]
    public SABnzbdQueue? Queue { get; set; }
}

public class SABnzbdHistoryResponse
{
    [JsonPropertyName("history")]
    public SABnzbdHistory? History { get; set; }
}
