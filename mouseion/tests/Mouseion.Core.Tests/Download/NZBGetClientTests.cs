// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Mouseion.Core.Download.Clients.NZBGet;

namespace Mouseion.Core.Tests.Download;

public class NZBGetClientTests
{
    [Fact]
    public void Settings_DefaultValues()
    {
        var settings = new NZBGetSettings();
        Assert.Equal("localhost", settings.Host);
        Assert.Equal(6789, settings.Port);
        Assert.Equal("nzbget", settings.Username);
        Assert.Equal("mouseion", settings.Category);
        Assert.False(settings.UseSsl);
    }

    [Fact]
    public void QueueItem_FileSizeCalculation_LargeFile()
    {
        var item = new NZBGetQueueItem
        {
            FileSizeHi = 1,
            FileSizeLo = 500
        };

        Assert.Equal(4294967796L, item.FileSize);
    }

    [Fact]
    public void QueueItem_FileSizeCalculation_SmallFile()
    {
        var item = new NZBGetQueueItem
        {
            FileSizeHi = 0,
            FileSizeLo = 1048576
        };

        Assert.Equal(1048576L, item.FileSize);
    }

    [Fact]
    public void QueueItem_RemainingCalculation()
    {
        var item = new NZBGetQueueItem
        {
            RemainingSizeHi = 0,
            RemainingSizeLo = 524288
        };

        Assert.Equal(524288L, item.RemainingSize);
    }

    [Fact]
    public void HistoryItem_SuccessStatus()
    {
        var item = new NZBGetHistoryItem
        {
            Status = "SUCCESS/ALL",
            DestDir = "/downloads/nzbget/completed"
        };

        Assert.True(item.Status.StartsWith("SUCCESS", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void HistoryItem_FailureStatus()
    {
        var item = new NZBGetHistoryItem
        {
            Status = "FAILURE/UNPACK",
            ParStatus = "SUCCESS",
            UnpackStatus = "FAILURE"
        };

        Assert.True(item.Status.StartsWith("FAILURE", StringComparison.OrdinalIgnoreCase));
        Assert.Equal("FAILURE", item.UnpackStatus);
    }

    [Fact]
    public void JsonRpcRequest_DefaultStructure()
    {
        var request = new JsonRpcRequest();

        Assert.Equal("2.0", request.JsonRpc);
        Assert.Equal(1, request.Id);
        Assert.Empty(request.Params);
    }

    [Fact]
    public void JsonRpcRequest_WithParams()
    {
        var request = new JsonRpcRequest
        {
            Method = "listgroups",
            Params = new object[] { false }
        };

        Assert.Equal("listgroups", request.Method);
        Assert.Single(request.Params);
    }

    [Fact]
    public void NZBGetStatus_Properties()
    {
        var status = new NZBGetStatus
        {
            ServerStandBy = false,
            DownloadRate = 15000000,
            DownloadPaused = false,
            FreeDiskSpaceMB = 50000
        };

        Assert.False(status.ServerStandBy);
        Assert.Equal(15000000, status.DownloadRate);
        Assert.Equal(50000, status.FreeDiskSpaceMB);
    }
}
