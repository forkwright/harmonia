// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Jobs;

public interface IScheduledTask
{
    string Name { get; }
    TimeSpan Interval { get; }
    Task ExecuteAsync(CancellationToken cancellationToken);
}
