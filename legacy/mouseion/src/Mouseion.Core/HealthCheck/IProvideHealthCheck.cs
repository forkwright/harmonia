// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.HealthCheck;

public interface IProvideHealthCheck
{
    HealthCheck Check();
}
