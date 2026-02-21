// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Mouseion.Core.Download.Clients.Transmission;

namespace Mouseion.Core.Tests.Download;

public class TransmissionClientTests
{
    [Fact]
    public void Settings_DefaultValues()
    {
        var settings = new TransmissionSettings();
        Assert.Equal("localhost", settings.Host);
        Assert.Equal(9091, settings.Port);
        Assert.Equal("/transmission/rpc", settings.UrlBase);
        Assert.Equal("mouseion", settings.Category);
        Assert.False(settings.UseSsl);
    }

    [Fact]
    public void TransmissionTorrentStatus_Constants()
    {
        Assert.Equal(0, TransmissionTorrentStatus.Stopped);
        Assert.Equal(1, TransmissionTorrentStatus.CheckPending);
        Assert.Equal(2, TransmissionTorrentStatus.Checking);
        Assert.Equal(3, TransmissionTorrentStatus.DownloadPending);
        Assert.Equal(4, TransmissionTorrentStatus.Downloading);
        Assert.Equal(5, TransmissionTorrentStatus.SeedPending);
        Assert.Equal(6, TransmissionTorrentStatus.Seeding);
    }

    [Fact]
    public void TransmissionTorrent_DefaultLabels_IsEmpty()
    {
        var torrent = new TransmissionTorrent();
        Assert.Empty(torrent.Labels);
        Assert.Equal(string.Empty, torrent.HashString);
    }

    [Fact]
    public void TransmissionTorrent_NegativeEta()
    {
        var torrent = new TransmissionTorrent { Eta = -1 };
        Assert.True(torrent.Eta < 0);
    }

    [Fact]
    public void TransmissionTorrent_LargeEta()
    {
        var torrent = new TransmissionTorrent { Eta = 400 * 24 * 3600 }; // > 1 year
        Assert.True(torrent.Eta > 365 * 24 * 3600);
    }

    [Fact]
    public void TransmissionSessionInfo_Defaults()
    {
        var info = new TransmissionSessionInfo();
        Assert.Equal(string.Empty, info.DownloadDir);
        Assert.Equal(string.Empty, info.Version);
        Assert.Equal(0, info.RpcVersion);
    }

    [Fact]
    public void TransmissionRequest_Structure()
    {
        var request = new TransmissionRequest
        {
            Method = "torrent-get",
            Arguments = new { fields = new[] { "id", "name" } },
            Tag = 42
        };

        Assert.Equal("torrent-get", request.Method);
        Assert.NotNull(request.Arguments);
        Assert.Equal(42, request.Tag);
    }
}
