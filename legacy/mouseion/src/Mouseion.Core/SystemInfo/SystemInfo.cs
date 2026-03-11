// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.SystemInfo;

public class SystemInfo
{
    public string Version { get; set; } = string.Empty;
    public string RuntimeVersion { get; set; } = string.Empty;
    public string OsName { get; set; } = string.Empty;
    public string OsVersion { get; set; } = string.Empty;
    public string Architecture { get; set; } = string.Empty;
    public DateTime StartTime { get; set; }
    public string PackageVersion { get; set; } = string.Empty;
    public string PackageAuthor { get; set; } = string.Empty;
    public string Branch { get; set; } = string.Empty;
}
