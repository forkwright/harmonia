// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// SignalR hub for real-time messages
using Microsoft.AspNetCore.SignalR;
using Mouseion.Common.EnvironmentInfo;
using Serilog;

namespace Mouseion.SignalR;

public class MessageHub : Hub
{
    private readonly ILogger _logger;

    public MessageHub()
    {
        _logger = Log.ForContext<MessageHub>();
    }

    public override async Task OnConnectedAsync()
    {
        await base.OnConnectedAsync();
        _logger.Debug("SignalR client connected: {ConnectionId}", Context.ConnectionId);

        await Clients.Caller.SendAsync("receiveMessage", new SignalRMessage
        {
            Name = "version",
            Body = new
            {
                version = BuildInfo.Version.ToString(),
                branch = BuildInfo.Branch,
                release = BuildInfo.Release,
                buildTime = BuildInfo.BuildDateTime
            }
        });
    }

    public override async Task OnDisconnectedAsync(Exception? exception)
    {
        _logger.Debug("SignalR client disconnected: {ConnectionId}", Context.ConnectionId);
        await base.OnDisconnectedAsync(exception);
    }
}
