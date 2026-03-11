// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Api.Tests;

public abstract class ControllerTestBase : IClassFixture<TestWebApplicationFactory>
{
    protected readonly HttpClient Client;

    protected ControllerTestBase(TestWebApplicationFactory factory)
    {
        Client = factory.CreateClient();
        Client.DefaultRequestHeaders.Add("X-Api-Key", "test-api-key");
    }
}
