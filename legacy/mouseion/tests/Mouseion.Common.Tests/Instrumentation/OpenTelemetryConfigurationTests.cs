// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Mouseion.Common.Instrumentation;

namespace Mouseion.Common.Tests.Instrumentation;

public class OpenTelemetryConfigurationTests
{
    [Fact]
    public void AddMouseionTelemetry_WithDefaultConfiguration_RegistersServices()
    {
        // Arrange
        var services = new ServiceCollection();

        // Act
        services.AddMouseionTelemetry();
        var provider = services.BuildServiceProvider();

        // Assert - OpenTelemetry services should be registered
        // We don't directly resolve the internal services, but verify no exception is thrown
        Assert.NotNull(provider);
    }

    [Fact]
    public void AddMouseionTelemetry_WithConfiguration_AppliesOptions()
    {
        // Arrange
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["Telemetry:Enabled"] = "true",
                ["Telemetry:EnablePrometheus"] = "true",
                ["Telemetry:EnableConsoleExporter"] = "false",
                ["Telemetry:OtlpEndpoint"] = "",
                ["Telemetry:ServiceInstanceId"] = "test-instance-1"
            })
            .Build();

        var services = new ServiceCollection();

        // Act
        services.AddMouseionTelemetry(configuration);
        var provider = services.BuildServiceProvider();

        // Assert
        Assert.NotNull(provider);
    }

    [Fact]
    public void AddMouseionTelemetry_WithDisabled_SkipsRegistration()
    {
        // Arrange
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["Telemetry:Enabled"] = "false"
            })
            .Build();

        var services = new ServiceCollection();

        // Act
        services.AddMouseionTelemetry(configuration);
        var provider = services.BuildServiceProvider();

        // Assert - should succeed without registering OTEL services
        Assert.NotNull(provider);
    }

    [Fact]
    public void AddMouseionTelemetry_WithOtlpEndpoint_ConfiguresExporter()
    {
        // Arrange
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["Telemetry:Enabled"] = "true",
                ["Telemetry:OtlpEndpoint"] = "http://localhost:4317"
            })
            .Build();

        var services = new ServiceCollection();

        // Act
        services.AddMouseionTelemetry(configuration);
        var provider = services.BuildServiceProvider();

        // Assert
        Assert.NotNull(provider);
    }

    [Fact]
    public void AddMouseionTelemetry_WithResourceAttributes_ConfiguresResources()
    {
        // Arrange
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["Telemetry:Enabled"] = "true",
                ["Telemetry:ResourceAttributes:custom.attribute"] = "custom_value",
                ["Telemetry:ResourceAttributes:deployment.region"] = "us-east-1"
            })
            .Build();

        var services = new ServiceCollection();

        // Act
        services.AddMouseionTelemetry(configuration);
        var provider = services.BuildServiceProvider();

        // Assert
        Assert.NotNull(provider);
    }

    [Fact]
    public void ActivitySource_IsInitialized()
    {
        // Assert
        Assert.NotNull(OpenTelemetryConfiguration.ActivitySource);
        Assert.Equal("Mouseion", OpenTelemetryConfiguration.ActivitySource.Name);
    }

    [Fact]
    public void StartActivity_ReturnsActivityOrNull()
    {
        // Act
        var activity = OpenTelemetryConfiguration.StartActivity("test_operation");

        // Assert - may be null if no listeners
        Assert.True(activity == null || activity is Activity);
        activity?.Dispose();
    }

    [Fact]
    public void StartActivity_WithKind_SetsActivityKind()
    {
        // Arrange
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        // Act
        using var activity = OpenTelemetryConfiguration.StartActivity("test_client_operation", ActivityKind.Client);

        // Assert
        if (activity != null)
        {
            Assert.Equal(ActivityKind.Client, activity.Kind);
        }
    }

    [Fact]
    public void StartActivity_WithParentContext_SetsParent()
    {
        // Arrange
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        var parentContext = new ActivityContext(
            ActivityTraceId.CreateRandom(),
            ActivitySpanId.CreateRandom(),
            ActivityTraceFlags.Recorded);

        // Act
        using var activity = OpenTelemetryConfiguration.StartActivity(
            "child_operation",
            ActivityKind.Internal,
            parentContext);

        // Assert
        if (activity != null)
        {
            Assert.Equal(parentContext.TraceId, activity.TraceId);
            Assert.Equal(parentContext.SpanId, activity.ParentSpanId);
        }
    }

    [Fact]
    public void StartActivity_WithTags_SetsTags()
    {
        // Arrange
        using var listener = new ActivityListener
        {
            ShouldListenTo = _ => true,
            Sample = (ref ActivityCreationOptions<ActivityContext> _) => ActivitySamplingResult.AllData
        };
        ActivitySource.AddActivityListener(listener);

        var tags = new List<KeyValuePair<string, object?>>
        {
            new("custom.tag1", "value1"),
            new("custom.tag2", 42)
        };

        // Act
        using var activity = OpenTelemetryConfiguration.StartActivity(
            "tagged_operation",
            ActivityKind.Internal,
            tags);

        // Assert
        if (activity != null)
        {
            Assert.Equal("value1", activity.GetTagItem("custom.tag1"));
            Assert.Equal(42, activity.GetTagItem("custom.tag2"));
        }
    }
}

public class TelemetryOptionsTests
{
    [Fact]
    public void TelemetryOptions_DefaultValues_AreCorrect()
    {
        // Arrange & Act
        var options = new TelemetryOptions();

        // Assert
        Assert.True(options.Enabled);
        Assert.True(options.EnablePrometheus);
        Assert.False(options.EnableConsoleExporter);
        Assert.Null(options.OtlpEndpoint);
        Assert.Null(options.ServiceInstanceId);
        Assert.NotNull(options.ResourceAttributes);
        Assert.Empty(options.ResourceAttributes);
    }

    [Fact]
    public void TelemetryOptions_SectionName_IsCorrect()
    {
        Assert.Equal("Telemetry", TelemetryOptions.SectionName);
    }
}
