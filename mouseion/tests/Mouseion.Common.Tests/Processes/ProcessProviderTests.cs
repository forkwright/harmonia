// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Processes;
using Serilog;
using Serilog.Core;

namespace Mouseion.Common.Tests.Processes;

public class ProcessProviderTests
{
    [Fact]
    public void BuildBatchCommandLine_WithPathAndArgs_FormatsCorrectly()
    {
        var path = @"C:\scripts\test.bat";
        var args = "arg1 arg2";

        var method = typeof(ProcessProvider).GetMethod("BuildBatchCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("cmd.exe", result.Path);
        Assert.Contains("/c", result.Args);
        Assert.Contains("test.bat", result.Args);
        Assert.Contains("arg1 arg2", result.Args);
    }

    [Fact]
    public void BuildBatchCommandLine_WithNullArgs_HandlesGracefully()
    {
        var path = @"C:\scripts\test.bat";
        string? args = null;

        var method = typeof(ProcessProvider).GetMethod("BuildBatchCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object?[] { path, args })!;

        Assert.Equal("cmd.exe", result.Path);
        Assert.Contains("/c", result.Args);
        Assert.Contains("test.bat", result.Args);
    }

    [Fact]
    public void BuildBatchCommandLine_WithSpecialCharacters_EscapesCorrectly()
    {
        var path = @"C:\scripts\test file.bat";
        var args = "arg&with|special<chars>";

        var method = typeof(ProcessProvider).GetMethod("BuildBatchCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("cmd.exe", result.Path);
        Assert.Contains("^&", result.Args);
        Assert.Contains("^|", result.Args);
        Assert.Contains("^<", result.Args);
        Assert.Contains("^>", result.Args);
    }

    [Fact]
    public void BuildPowerShellCommandLine_WithPathAndArgs_FormatsCorrectly()
    {
        var path = @"C:\scripts\test.ps1";
        var args = "-Param1 value1";

        var method = typeof(ProcessProvider).GetMethod("BuildPowerShellCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("powershell.exe", result.Path);
        Assert.Contains("-NoProfile", result.Args);
        Assert.Contains("-File", result.Args);
        Assert.Contains("test.ps1", result.Args);
    }

    [Fact]
    public void BuildPowerShellCommandLine_WithNullArgs_HandlesGracefully()
    {
        var path = @"C:\scripts\test.ps1";
        string? args = null;

        var method = typeof(ProcessProvider).GetMethod("BuildPowerShellCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object?[] { path, args })!;

        Assert.Equal("powershell.exe", result.Path);
        Assert.Contains("-NoProfile", result.Args);
        Assert.Contains("-File", result.Args);
    }

    [Fact]
    public void BuildPowerShellCommandLine_WithSingleQuotes_EscapesCorrectly()
    {
        var path = @"C:\scripts\test's script.ps1";
        var args = "-Message 'Hello World'";

        var method = typeof(ProcessProvider).GetMethod("BuildPowerShellCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("powershell.exe", result.Path);
        Assert.Contains("''", result.Args);
    }

    [Fact]
    public void BuildPythonCommandLine_WithPathAndArgs_FormatsCorrectly()
    {
        var path = @"C:\scripts\test.py";
        var args = "--verbose";

        var method = typeof(ProcessProvider).GetMethod("BuildPythonCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("python.exe", result.Path);
        Assert.Contains("test.py", result.Args);
        Assert.Contains("--verbose", result.Args);
    }

    [Fact]
    public void BuildPythonCommandLine_WithNullArgs_HandlesGracefully()
    {
        var path = @"C:\scripts\test.py";
        string? args = null;

        var method = typeof(ProcessProvider).GetMethod("BuildPythonCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object?[] { path, args })!;

        Assert.Equal("python.exe", result.Path);
        Assert.Contains("test.py", result.Args);
    }

    [Fact]
    public void BuildPythonCommandLine_WithSpacesInPath_QuotesCorrectly()
    {
        var path = @"C:\my scripts\test file.py";
        var args = "arg1";

        var method = typeof(ProcessProvider).GetMethod("BuildPythonCommandLine",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = ((string Path, string Args))method!.Invoke(null, new object[] { path, args })!;

        Assert.Equal("python.exe", result.Path);
        Assert.Contains("\"", result.Args);
    }

    [Fact]
    public void EscapeCommandLineArg_WithSpecialCharacters_EscapesAll()
    {
        var arg = "test&pipe|redirect<output>and^percent%";

        var method = typeof(ProcessProvider).GetMethod("EscapeCommandLineArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.Contains("^&", result);
        Assert.Contains("^|", result);
        Assert.Contains("^<", result);
        Assert.Contains("^>", result);
        Assert.Contains("^^", result);
        Assert.Contains("^%", result);
    }

    [Fact]
    public void EscapeCommandLineArg_WithQuotes_DoublesQuotes()
    {
        var arg = "test \"quoted\" string";

        var method = typeof(ProcessProvider).GetMethod("EscapeCommandLineArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.Contains("\"\"", result);
    }

    [Fact]
    public void EscapeCommandLineArg_WithSpaces_WrapsInQuotes()
    {
        var arg = "test with spaces";

        var method = typeof(ProcessProvider).GetMethod("EscapeCommandLineArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.StartsWith("\"", result);
        Assert.EndsWith("\"", result);
    }

    [Fact]
    public void EscapeCommandLineArg_WithEmptyString_ReturnsEmptyQuotes()
    {
        var arg = "";

        var method = typeof(ProcessProvider).GetMethod("EscapeCommandLineArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.Equal("\"\"", result);
    }

    [Fact]
    public void EscapePowerShellArg_WithSingleQuotes_DoublesQuotes()
    {
        var arg = "test's value";

        var method = typeof(ProcessProvider).GetMethod("EscapePowerShellArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.Contains("''", result);
        Assert.StartsWith("'", result);
        Assert.EndsWith("'", result);
    }

    [Fact]
    public void EscapePowerShellArg_WithNoSpecialChars_WrapsInSingleQuotes()
    {
        var arg = "test value";

        var method = typeof(ProcessProvider).GetMethod("EscapePowerShellArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.StartsWith("'", result);
        Assert.EndsWith("'", result);
        Assert.Equal("'test value'", result);
    }

    [Fact]
    public void EscapePowerShellArg_WithEmptyString_ReturnsEmptyQuotes()
    {
        var arg = "";

        var method = typeof(ProcessProvider).GetMethod("EscapePowerShellArg",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
        var result = (string)method!.Invoke(null, new object[] { arg })!;

        Assert.Equal("''", result);
    }
}
