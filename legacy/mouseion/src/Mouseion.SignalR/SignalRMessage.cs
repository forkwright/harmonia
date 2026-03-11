// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// SignalR message DTO
namespace Mouseion.SignalR;

public class SignalRMessage
{
    public string Name { get; set; } = string.Empty;
    public object? Body { get; set; }
}
