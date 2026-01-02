// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.EnsureThat
{
    internal static class ExceptionMessages
    {
        internal const string EnsureExtensions_IsEmptyCollection = "Empty collection is not allowed.";
        internal const string EnsureExtensions_IsEmptyGuid = "Empty Guid is not allowed.";
        internal const string EnsureExtensions_IsNotFalse = "Expected an expression that evaluates to false.";
        internal const string EnsureExtensions_IsNotGt = "value '{0}' is not greater than limit '{1}'.";
        internal const string EnsureExtensions_IsNotGte = "value '{0}' is not greater than or equal to limit '{1}'.";
        internal const string EnsureExtensions_IsNotInRange_ToHigh = "value '{0}' is > max '{1}'.";
        internal const string EnsureExtensions_IsNotInRange_ToLong = "The string is too long. Must be between '{0}' and '{1}' but was '{2}' characters long.";
        internal const string EnsureExtensions_IsNotInRange_ToLow = "value '{0}' is < min '{1}'.";
        internal const string EnsureExtensions_IsNotInRange_ToShort = "The string is not long enough. Must be between '{0}' and '{1}' but was '{2}' characters long.";
        internal const string EnsureExtensions_IsNotLt = "value '{0}' is not lower than limit '{1}'.";
        internal const string EnsureExtensions_IsNotLte = "value '{0}' is not lower than or equal to limit '{1}'.";
        internal const string EnsureExtensions_IsNotNull = "Value can not be null.";
        internal const string EnsureExtensions_IsNotNullOrEmpty = "The string can't be null or empty.";
        internal const string EnsureExtensions_IsNotNullOrWhiteSpace = "The string can't be left empty, null or consist of only whitespaces.";
        internal const string EnsureExtensions_IsNotOfType = "The param is not of expected type: '{0}'.";
        internal const string EnsureExtensions_IsNotTrue = "Expected an expression that evaluates to true.";
        internal const string EnsureExtensions_NoMatch = "value '{0}' does not match '{1}'";
        internal const string ExpressionUtils_GetRightMostMember_NoMemberFound = "No MemberExpression found in expression: '{0}'.";
    }
}
