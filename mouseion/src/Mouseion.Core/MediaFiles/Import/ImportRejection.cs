// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles.Import;

public class ImportRejection
{
    public ImportRejectionReason Reason { get; }
    public string Message { get; }

    public ImportRejection(ImportRejectionReason reason, string message)
    {
        Reason = reason;
        Message = message;
    }
}
