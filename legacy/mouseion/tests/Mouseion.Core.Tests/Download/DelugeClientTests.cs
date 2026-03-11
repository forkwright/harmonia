// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Download;
using Mouseion.Core.Download.Clients.Deluge;

namespace Mouseion.Core.Tests.Download;

public class DelugeClientTests
{
    [Fact]
    public void Settings_DefaultValues()
    {
        var settings = new DelugeSettings();
        Assert.Equal("localhost", settings.Host);
        Assert.Equal(8112, settings.Port);
        Assert.Equal("deluge", settings.Password);
        Assert.Equal("mouseion", settings.Category);
        Assert.False(settings.UseSsl);
    }

    [Fact]
    public void DelugeTorrent_Seeding()
    {
        var torrent = new DelugeTorrent
        {
            Hash = "abc123def456",
            Name = "Test Torrent",
            State = "Seeding",
            TotalSize = 1073741824,
            Progress = 100.0,
            IsFinished = true
        };

        Assert.Equal("Seeding", torrent.State);
        Assert.True(torrent.IsFinished);
        Assert.Equal(100.0, torrent.Progress);
    }

    [Fact]
    public void DelugeTorrent_Downloading()
    {
        var torrent = new DelugeTorrent
        {
            State = "Downloading",
            TotalSize = 2147483648,
            TotalRemaining = 1073741824,
            Progress = 50.0,
            IsFinished = false
        };

        Assert.Equal("Downloading", torrent.State);
        Assert.False(torrent.IsFinished);
        Assert.Equal(1073741824, torrent.TotalRemaining);
    }

    [Fact]
    public void DelugeTorrent_Error_WithMessage()
    {
        var torrent = new DelugeTorrent
        {
            State = "Error",
            Message = "Tracker is down"
        };

        Assert.Equal("Error", torrent.State);
        Assert.Equal("Tracker is down", torrent.Message);
    }

    [Fact]
    public void DelugeTorrent_Paused_Finished()
    {
        var torrent = new DelugeTorrent
        {
            State = "Paused",
            Paused = true,
            IsFinished = true
        };

        Assert.True(torrent.Paused);
        Assert.True(torrent.IsFinished);
    }

    [Fact]
    public void DelugeTorrent_NegativeEta()
    {
        var torrent = new DelugeTorrent { Eta = -1 };
        Assert.True(torrent.Eta < 0);
    }

    [Fact]
    public void DelugeJsonRpcRequest_Structure()
    {
        var request = new DelugeJsonRpcRequest
        {
            Method = "auth.login",
            Params = new object[] { "password" },
            Id = 1
        };

        Assert.Equal("auth.login", request.Method);
        Assert.Single(request.Params);
        Assert.Equal(1, request.Id);
    }

    [Fact]
    public void DelugeJsonRpcResponse_WithError()
    {
        var response = new DelugeJsonRpcResponse<bool>
        {
            Error = new DelugeError { Code = -1, Message = "Not authenticated" }
        };

        Assert.NotNull(response.Error);
        Assert.Equal(-1, response.Error.Code);
    }

    [Fact]
    public void DelugeJsonRpcResponse_Success()
    {
        var response = new DelugeJsonRpcResponse<bool>
        {
            Result = true,
            Error = null
        };

        Assert.True(response.Result);
        Assert.Null(response.Error);
    }
}
