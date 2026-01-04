// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Runtime.InteropServices;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Core.SystemInfo;

public interface ISystemService
{
    SystemInfo GetSystemInfo();
}

public class SystemService : ISystemService
{
    private readonly DateTime _startTime;

    public SystemService()
    {
        _startTime = DateTime.UtcNow;
    }

    public SystemInfo GetSystemInfo()
    {
        return new SystemInfo
        {
            Version = BuildInfo.Version.ToString(),
            RuntimeVersion = Environment.Version.ToString(),
            OsName = OsInfo.Os.ToString(),
            OsVersion = Environment.OSVersion.VersionString,
            Architecture = RuntimeInformation.OSArchitecture.ToString(),
            StartTime = _startTime,
            PackageVersion = BuildInfo.Version.ToString(),
            PackageAuthor = "Mouseion Contributors",
            Branch = BuildInfo.Branch
        };
    }
}
