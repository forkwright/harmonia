// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Mouseion.Core.Download.Clients.SABnzbd;

namespace Mouseion.Core.Tests.Download;

public class SABnzbdClientTests
{
    [Fact]
    public void Settings_DefaultValues()
    {
        var settings = new SABnzbdSettings();
        Assert.Equal("localhost", settings.Host);
        Assert.Equal(8080, settings.Port);
        Assert.Equal("mouseion", settings.Category);
        Assert.False(settings.UseSsl);
        Assert.Equal(string.Empty, settings.ApiKey);
    }

    [Fact]
    public void QueueItem_Properties()
    {
        var item = new SABnzbdQueueItem
        {
            NzoId = "SABnzbd_nzo_abc",
            FileName = "test.nzb",
            Category = "mouseion",
            TotalMb = "1024.5",
            RemainingMb = "512.3",
            Status = "Downloading",
            Percentage = "50"
        };

        Assert.Equal("SABnzbd_nzo_abc", item.NzoId);
        Assert.Equal("1024.5", item.TotalMb);
        Assert.Equal("512.3", item.RemainingMb);
        Assert.Equal("50", item.Percentage);
    }

    [Fact]
    public void HistoryItem_CompletedProperties()
    {
        var item = new SABnzbdHistoryItem
        {
            NzoId = "SABnzbd_nzo_def",
            Name = "completed.nzb",
            Status = "Completed",
            Bytes = 1073741824,
            Storage = "/downloads/completed/test"
        };

        Assert.Equal("Completed", item.Status);
        Assert.Equal(1073741824, item.Bytes);
        Assert.Equal("/downloads/completed/test", item.Storage);
    }

    [Fact]
    public void HistoryItem_FailedProperties()
    {
        var item = new SABnzbdHistoryItem
        {
            Status = "Failed",
            FailMessage = "CRC error in data"
        };

        Assert.Equal("Failed", item.Status);
        Assert.Equal("CRC error in data", item.FailMessage);
    }

    [Fact]
    public void Queue_DefaultEmpty()
    {
        var queue = new SABnzbdQueue();
        Assert.Empty(queue.Slots);
        Assert.Equal(string.Empty, queue.Status);
    }

    [Fact]
    public void History_DefaultEmpty()
    {
        var history = new SABnzbdHistory();
        Assert.Empty(history.Slots);
    }

    [Fact]
    public void Config_CategoryProperties()
    {
        var category = new SABnzbdCategory
        {
            Name = "mouseion",
            Dir = "/downloads/mouseion"
        };

        Assert.Equal("mouseion", category.Name);
        Assert.Equal("/downloads/mouseion", category.Dir);
    }
}
