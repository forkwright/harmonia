// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.ImportLists.Exceptions;

public class ImportListException : Exception
{
    public ImportListException(string message) : base(message) { }
    public ImportListException(string message, Exception innerException) : base(message, innerException) { }
    public ImportListException(string message, params object[] args) : base(string.Format(message, args)) { }
}
