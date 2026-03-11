// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Jobs;

public class ScheduledTask : ModelBase
{
    public string TypeName { get; set; } = string.Empty;
    public int Interval { get; set; }
    public DateTime LastExecution { get; set; }
    public DateTime LastStartTime { get; set; }
    public bool Enabled { get; set; } = true;
}
