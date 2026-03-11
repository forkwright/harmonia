// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.HealthCheck;

public abstract class HealthCheckBase : IProvideHealthCheck
{
    public abstract HealthCheck Check();

    public virtual bool CheckOnStartup => true;
    public virtual bool CheckOnSchedule => true;
}
