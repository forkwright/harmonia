// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Model;
using Serilog;

namespace Mouseion.Common.Processes
{
    public interface IProcessProvider
    {
        ProcessInfo? GetCurrentProcess();
        ProcessInfo? GetProcessById(int id);
        List<ProcessInfo> FindProcessByName(string name);
        void OpenDefaultBrowser(string url);
        void WaitForExit(Process process);
        void SetPriority(int processId, ProcessPriorityClass priority);
        void KillAll(string processName);
        void Kill(int processId);
        bool Exists(int processId);
        bool Exists(string processName);
        ProcessPriorityClass GetCurrentProcessPriority();
        Process Start(string path, string? args = null, StringDictionary? environmentVariables = null, Action<string>? onOutputDataReceived = null, Action<string>? onErrorDataReceived = null);
        Process SpawnNewProcess(string path, string? args = null, StringDictionary? environmentVariables = null, bool noWindow = false);
        ProcessOutput StartAndCapture(string path, string? args = null, StringDictionary? environmentVariables = null);
    }

    public class ProcessProvider : IProcessProvider
    {
        private readonly ILogger _logger;

        public const string MOUSEION_PROCESS_NAME = "Mouseion";
        public const string MOUSEION_CONSOLE_PROCESS_NAME = "Mouseion.Console";

        public ProcessProvider(ILogger logger)
        {
            _logger = logger;
        }

        public static int GetCurrentProcessId()
        {
            return Environment.ProcessId;
        }

        public ProcessInfo? GetCurrentProcess()
        {
            return ConvertToProcessInfo(Process.GetCurrentProcess());
        }

        public bool Exists(int processId)
        {
            return GetProcessById(processId) != null;
        }

        public bool Exists(string processName)
        {
            return GetProcessesByName(processName).Any();
        }

        public ProcessPriorityClass GetCurrentProcessPriority()
        {
            return Process.GetCurrentProcess().PriorityClass;
        }

        public ProcessInfo? GetProcessById(int id)
        {
            _logger.Debug("Finding process with Id:{Id}", id);

            var processInfo = ConvertToProcessInfo(Process.GetProcesses().FirstOrDefault(p => p.Id == id));

            if (processInfo == null)
            {
                _logger.Warning("Unable to find process with ID {Id}", id);
            }
            else
            {
                _logger.Debug("Found process {ProcessInfo}", processInfo.ToString());
            }

            return processInfo;
        }

        public List<ProcessInfo> FindProcessByName(string name)
        {
            return GetProcessesByName(name).Select(ConvertToProcessInfo).Where(c => c != null).Cast<ProcessInfo>().ToList();
        }

        public void OpenDefaultBrowser(string url)
        {
            _logger.Information("Opening URL [{Url}]", url);

            var process = new Process
            {
                StartInfo = new ProcessStartInfo(url)
                {
                    UseShellExecute = true
                }
            };

            process.Start();
        }

        public Process Start(string path, string? args = null, StringDictionary? environmentVariables = null, Action<string>? onOutputDataReceived = null, Action<string>? onErrorDataReceived = null)
        {
            (path, args) = GetPathAndArgs(path, args);

            var processLogger = Log.ForContext("ProcessName", new FileInfo(path).Name);

            var startInfo = new ProcessStartInfo(path, args ?? string.Empty)
            {
                CreateNoWindow = true,
                UseShellExecute = false,
                RedirectStandardError = true,
                RedirectStandardOutput = true,
                RedirectStandardInput = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8
            };

            if (environmentVariables != null)
            {
                foreach (DictionaryEntry environmentVariable in environmentVariables)
                {
                    try
                    {
                        _logger.Verbose("Setting environment variable '{Key}' to '{Value}'", environmentVariable.Key, environmentVariable.Value);

                        var key = environmentVariable.Key.ToString()!;
                        var value = environmentVariable.Value?.ToString();

                        startInfo.EnvironmentVariables[key] = value;
                    }
                    catch (ArgumentException ex)
                    {
                        if (environmentVariable.Value == null)
                        {
                            _logger.Error(ex, "Unable to set environment variable '{Key}', value is null", environmentVariable.Key);
                        }
                        else
                        {
                            _logger.Error(ex, "Unable to set environment variable '{Key}' (invalid argument)", environmentVariable.Key);
                        }

                        throw;
                    }
                    catch (InvalidOperationException ex)
                    {
                        _logger.Error(ex, "Unable to set environment variable '{Key}' (invalid operation)", environmentVariable.Key);
                        throw;
                    }
                }
            }

            processLogger.Debug("Starting {Path} {Args}", path, args);

            var process = new Process
            {
                StartInfo = startInfo
            };

            process.OutputDataReceived += (sender, eventArgs) =>
            {
                if (string.IsNullOrWhiteSpace(eventArgs.Data))
                {
                    return;
                }

                processLogger.Debug("{Output}", eventArgs.Data);

                onOutputDataReceived?.Invoke(eventArgs.Data);
            };

            process.ErrorDataReceived += (sender, eventArgs) =>
            {
                if (string.IsNullOrWhiteSpace(eventArgs.Data))
                {
                    return;
                }

                processLogger.Error("{Error}", eventArgs.Data);

                onErrorDataReceived?.Invoke(eventArgs.Data);
            };

            process.Start();

            process.BeginErrorReadLine();
            process.BeginOutputReadLine();

            return process;
        }

        public Process SpawnNewProcess(string path, string? args = null, StringDictionary? environmentVariables = null, bool noWindow = false)
        {
            (path, args) = GetPathAndArgs(path, args);

            _logger.Debug("Starting {Path} {Args}", path, args);

            var startInfo = new ProcessStartInfo(path, args ?? string.Empty);
            startInfo.CreateNoWindow = noWindow;
            startInfo.UseShellExecute = !noWindow;

            var process = new Process
            {
                StartInfo = startInfo
            };

            process.Start();

            return process;
        }

        public ProcessOutput StartAndCapture(string path, string? args = null, StringDictionary? environmentVariables = null)
        {
            var output = new ProcessOutput();
            var process = Start(path,
                                args,
                                environmentVariables,
                                s => output.Lines.Add(new ProcessOutputLine(ProcessOutputLevel.Standard, s)),
                                error => output.Lines.Add(new ProcessOutputLine(ProcessOutputLevel.Error, error)));

            process.WaitForExit();
            output.ExitCode = process.ExitCode;

            return output;
        }

        public void WaitForExit(Process process)
        {
            _logger.Debug("Waiting for process {ProcessName} to exit.", process.ProcessName);

            process.WaitForExit();
        }

        public void SetPriority(int processId, ProcessPriorityClass priority)
        {
            var process = Process.GetProcessById(processId);

            _logger.Information("Updating [{ProcessName}] process priority from {OldPriority} to {NewPriority}",
                        process.ProcessName,
                        process.PriorityClass,
                        priority);

            process.PriorityClass = priority;
        }

        public void Kill(int processId)
        {
            var process = Process.GetProcesses().FirstOrDefault(p => p.Id == processId);

            if (process == null)
            {
                _logger.Warning("Cannot find process with id: {ProcessId}", processId);
                return;
            }

            process.Refresh();

            if (process.Id != GetCurrentProcessId() && process.HasExited)
            {
                _logger.Debug("Process has already exited");
                return;
            }

            _logger.Information("[{ProcessId}]: Killing process", process.Id);
            process.Kill();
            _logger.Information("[{ProcessId}]: Waiting for exit", process.Id);
            process.WaitForExit();
            _logger.Information("[{ProcessId}]: Process terminated successfully", process.Id);
        }

        public void KillAll(string processName)
        {
            var processes = GetProcessesByName(processName);

            _logger.Debug("Found {Count} processes to kill", processes.Count);

            foreach (var processInfo in processes)
            {
                if (processInfo.Id == GetCurrentProcessId())
                {
                    _logger.Debug("Tried killing own process, skipping: {ProcessId} [{ProcessName}]", processInfo.Id, processInfo.ProcessName);
                    continue;
                }

                _logger.Debug("Killing process: {ProcessId} [{ProcessName}]", processInfo.Id, processInfo.ProcessName);
                Kill(processInfo.Id);
            }
        }

        private ProcessInfo? ConvertToProcessInfo(Process? process)
        {
            if (process == null)
            {
                return null;
            }

            process.Refresh();

            ProcessInfo? processInfo = null;

            try
            {
                if (process.Id <= 0)
                {
                    return null;
                }

                processInfo = new ProcessInfo();
                processInfo.Id = process.Id;
                processInfo.Name = process.ProcessName;
                processInfo.StartPath = process.MainModule?.FileName;

                if (process.Id != GetCurrentProcessId() && process.HasExited)
                {
                    processInfo = null;
                }
            }
            catch (Win32Exception e)
            {
                _logger.Warning(e, "Couldn't get process info for {ProcessName}", process.ProcessName);
            }

            return processInfo;
        }

        private List<Process> GetProcessesByName(string name)
        {
            var processes = Process.GetProcessesByName(name).ToList();

            _logger.Debug("Found {Count} processes with the name: {Name}", processes.Count, name);

            try
            {
                foreach (var process in processes)
                {
                    _logger.Debug(" - [{ProcessId}] {ProcessName}", process.Id, process.ProcessName);
                }
            }
            catch (InvalidOperationException)
            {
                // Process may have exited while enumerating - safe to ignore
            }
            catch (System.ComponentModel.Win32Exception)
            {
                // Access denied to process info - safe to ignore for logging
            }

            return processes;
        }

        private (string Path, string? Args) GetPathAndArgs(string path, string? args)
        {
            if (!OsInfo.IsWindows)
            {
                return (path, args);
            }

            if (path.EndsWith(".bat", StringComparison.InvariantCultureIgnoreCase))
            {
                return BuildBatchCommandLine(path, args);
            }

            if (path.EndsWith(".ps1", StringComparison.InvariantCultureIgnoreCase))
            {
                return BuildPowerShellCommandLine(path, args);
            }

            if (path.EndsWith(".py", StringComparison.InvariantCultureIgnoreCase))
            {
                return BuildPythonCommandLine(path, args);
            }

            return (path, args);
        }

        private static (string Path, string Args) BuildBatchCommandLine(string path, string? args)
        {
            var escapedPath = EscapeCommandLineArg(path);
            var escapedArgs = string.IsNullOrWhiteSpace(args) ? string.Empty : EscapeCommandLineArg(args);
            return ("cmd.exe", $"/c {escapedPath} {escapedArgs}");
        }

        private static (string Path, string Args) BuildPowerShellCommandLine(string path, string? args)
        {
            var escapedPath = EscapePowerShellArg(path);
            var escapedArgs = string.IsNullOrWhiteSpace(args) ? string.Empty : EscapePowerShellArg(args);
            return ("powershell.exe", $"-NoProfile -File {escapedPath} {escapedArgs}");
        }

        private static (string Path, string Args) BuildPythonCommandLine(string path, string? args)
        {
            var escapedPath = EscapeCommandLineArg(path);
            var escapedArgs = string.IsNullOrWhiteSpace(args) ? string.Empty : EscapeCommandLineArg(args);
            return ("python.exe", $"{escapedPath} {escapedArgs}");
        }

        /// <summary>
        /// Escapes a command-line argument for cmd.exe to prevent injection attacks
        /// </summary>
        private static string EscapeCommandLineArg(string arg)
        {
            if (string.IsNullOrWhiteSpace(arg))
            {
                return "\"\"";
            }

            // Escape special characters for cmd.exe: &, |, <, >, ^, ", and %
            // Also handle quotes by doubling them inside quoted strings
            var escaped = arg
                .Replace("\"", "\"\"")  // Double quotes
                .Replace("&", "^&")
                .Replace("|", "^|")
                .Replace("<", "^<")
                .Replace(">", "^>")
                .Replace("^", "^^")
                .Replace("%", "^%");

            // Wrap in quotes if it contains spaces or special characters
            if (escaped.Contains(" ") || escaped != arg)
            {
                return $"\"{escaped}\"";
            }

            return escaped;
        }

        /// <summary>
        /// Escapes a PowerShell argument to prevent injection attacks
        /// </summary>
        private static string EscapePowerShellArg(string arg)
        {
            if (string.IsNullOrWhiteSpace(arg))
            {
                return "''";
            }

            // PowerShell uses single quotes for literal strings (no escape sequences)
            // Double any single quotes and wrap in single quotes
            var escaped = arg.Replace("'", "''");
            return $"'{escaped}'";
        }
    }
}
