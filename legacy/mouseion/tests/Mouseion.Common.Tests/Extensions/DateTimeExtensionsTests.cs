// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class DateTimeExtensionsTests
{
    [Fact]
    public void InNextDays_should_return_true_for_tomorrow()
    {
        var tomorrow = DateTime.UtcNow.AddDays(1);

        Assert.True(tomorrow.InNextDays(2));
    }

    [Fact]
    public void InNextDays_should_return_false_for_past_date()
    {
        var yesterday = DateTime.UtcNow.AddDays(-1);

        Assert.False(yesterday.InNextDays(2));
    }

    [Fact]
    public void InNextDays_should_return_false_for_far_future()
    {
        var farFuture = DateTime.UtcNow.AddDays(10);

        Assert.False(farFuture.InNextDays(2));
    }

    [Fact]
    public void InLastDays_should_return_true_for_yesterday()
    {
        var yesterday = DateTime.UtcNow.AddDays(-1);

        Assert.True(yesterday.InLastDays(2));
    }

    [Fact]
    public void InLastDays_should_return_false_for_future_date()
    {
        var tomorrow = DateTime.UtcNow.AddDays(1);

        Assert.False(tomorrow.InLastDays(2));
    }

    [Fact]
    public void InLastDays_should_return_false_for_far_past()
    {
        var farPast = DateTime.UtcNow.AddDays(-10);

        Assert.False(farPast.InLastDays(2));
    }

    [Fact]
    public void InNext_should_return_true_for_time_within_span()
    {
        var inOneHour = DateTime.UtcNow.AddHours(1);

        Assert.True(inOneHour.InNext(TimeSpan.FromHours(2)));
    }

    [Fact]
    public void InNext_should_return_false_for_time_outside_span()
    {
        var inThreeHours = DateTime.UtcNow.AddHours(3);

        Assert.False(inThreeHours.InNext(TimeSpan.FromHours(2)));
    }

    [Fact]
    public void InLast_should_return_true_for_time_within_span()
    {
        var oneHourAgo = DateTime.UtcNow.AddHours(-1);

        Assert.True(oneHourAgo.InLast(TimeSpan.FromHours(2)));
    }

    [Fact]
    public void InLast_should_return_false_for_time_outside_span()
    {
        var threeHoursAgo = DateTime.UtcNow.AddHours(-3);

        Assert.False(threeHoursAgo.InLast(TimeSpan.FromHours(2)));
    }

    [Fact]
    public void Before_should_return_true_when_earlier()
    {
        var earlier = new DateTime(2020, 1, 1);
        var later = new DateTime(2020, 12, 31);

        Assert.True(earlier.Before(later));
    }

    [Fact]
    public void Before_should_return_true_when_equal()
    {
        var date = new DateTime(2020, 1, 1);

        Assert.True(date.Before(date));
    }

    [Fact]
    public void Before_should_return_false_when_later()
    {
        var earlier = new DateTime(2020, 1, 1);
        var later = new DateTime(2020, 12, 31);

        Assert.False(later.Before(earlier));
    }

    [Fact]
    public void After_should_return_true_when_later()
    {
        var earlier = new DateTime(2020, 1, 1);
        var later = new DateTime(2020, 12, 31);

        Assert.True(later.After(earlier));
    }

    [Fact]
    public void After_should_return_true_when_equal()
    {
        var date = new DateTime(2020, 1, 1);

        Assert.True(date.After(date));
    }

    [Fact]
    public void After_should_return_false_when_earlier()
    {
        var earlier = new DateTime(2020, 1, 1);
        var later = new DateTime(2020, 12, 31);

        Assert.False(earlier.After(later));
    }

    [Fact]
    public void Between_should_return_true_when_in_range()
    {
        var start = new DateTime(2020, 1, 1);
        var middle = new DateTime(2020, 6, 15);
        var end = new DateTime(2020, 12, 31);

        Assert.True(middle.Between(start, end));
    }

    [Fact]
    public void Between_should_return_true_on_boundaries()
    {
        var start = new DateTime(2020, 1, 1);
        var end = new DateTime(2020, 12, 31);

        Assert.True(start.Between(start, end));
        Assert.True(end.Between(start, end));
    }

    [Fact]
    public void Between_should_return_false_when_outside_range()
    {
        var start = new DateTime(2020, 1, 1);
        var end = new DateTime(2020, 12, 31);
        var outside = new DateTime(2021, 6, 15);

        Assert.False(outside.Between(start, end));
    }

    [Fact]
    public void Epoch_should_be_unix_epoch()
    {
        Assert.Equal(new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc), DateTimeExtensions.Epoch);
    }
}
