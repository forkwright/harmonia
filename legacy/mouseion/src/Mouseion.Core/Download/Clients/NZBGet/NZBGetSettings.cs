// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download.Clients.NZBGet;

public class NZBGetSettings
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 6789;
    public bool UseSsl { get; set; }
    public string UrlBase { get; set; } = string.Empty;
    public string Username { get; set; } = "nzbget";
    public string Password { get; set; } = string.Empty;
    public string Category { get; set; } = "mouseion";
}
