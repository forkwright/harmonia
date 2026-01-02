// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.RootFolders;

public class RootFolder : ModelBase
{
    public string Path { get; set; } = string.Empty;
    public MediaType MediaType { get; set; }
    public bool Accessible { get; set; } = true;
    public long? FreeSpace { get; set; }
    public long? TotalSpace { get; set; }
}
