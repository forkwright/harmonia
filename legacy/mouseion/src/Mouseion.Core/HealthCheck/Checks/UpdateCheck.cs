// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.HealthCheck.Checks;

public class UpdateCheck : IProvideHealthCheck
{
    public HealthCheck Check()
    {
        // Placeholder - update system not yet implemented
        return new HealthCheck(
            HealthCheckResult.Ok,
            "Update system not yet implemented"
        );
    }
}
