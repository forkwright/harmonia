// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Housekeeping;

public interface IHousekeepingTask
{
    string Name { get; }
    Task CleanAsync(CancellationToken cancellationToken = default);
}
