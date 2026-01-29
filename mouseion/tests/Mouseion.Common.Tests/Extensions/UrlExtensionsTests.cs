// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Extensions;

namespace Mouseion.Common.Tests.Extensions;

public class UrlExtensionsTests
{
    [Fact]
    public void IsValidUrl_should_return_true_for_http_url()
    {
        Assert.True("http://example.com".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_true_for_https_url()
    {
        Assert.True("https://example.com".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_true_for_url_with_path()
    {
        Assert.True("https://example.com/path/to/resource".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_true_for_url_with_query()
    {
        Assert.True("https://example.com?query=value".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_true_for_url_with_port()
    {
        Assert.True("https://example.com:8080".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_true_for_ftp_url()
    {
        Assert.True("ftp://files.example.com".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_null()
    {
        string? url = null;
        Assert.False(url.IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_empty()
    {
        Assert.False("".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_whitespace()
    {
        Assert.False("   ".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_leading_space()
    {
        Assert.False(" https://example.com".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_trailing_space()
    {
        Assert.False("https://example.com ".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_relative_url()
    {
        Assert.False("/path/to/resource".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_plain_text()
    {
        Assert.False("not a url".IsValidUrl());
    }

    [Fact]
    public void IsValidUrl_should_return_false_for_missing_scheme()
    {
        Assert.False("example.com".IsValidUrl());
    }
}
