// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Messaging;

namespace Mouseion.Core.HealthCheck.Events;

public class HealthCheckRestoredEvent : IEvent
{
    public HealthCheck HealthCheck { get; }
    public bool IsInStartupGracePeriod { get; }

    public HealthCheckRestoredEvent(HealthCheck healthCheck, bool isInStartupGracePeriod)
    {
        HealthCheck = healthCheck;
        IsInStartupGracePeriod = isInStartupGracePeriod;
    }
}
