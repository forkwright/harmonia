// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace Mouseion.Common.OAuth
{
    public static class OAuthTools
    {
        private const string AlphaNumeric = Upper + Lower + Digit;
        private const string Digit = "1234567890";
        private const string Lower = "abcdefghijklmnopqrstuvwxyz";
        private const string Unreserved = AlphaNumeric + "-._~";
        private const string Upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        private static readonly Random _random;
        private static readonly object _randomLock = new object();
        private static readonly RandomNumberGenerator _rng = RandomNumberGenerator.Create();
        private static readonly Encoding _encoding = Encoding.UTF8;

        static OAuthTools()
        {
            var bytes = new byte[4];
            _rng.GetNonZeroBytes(bytes);
            _random = new Random(BitConverter.ToInt32(bytes, 0));
        }

        public static string GetNonce()
        {
            const string chars = Lower + Digit;

            var nonce = new char[16];
            lock (_randomLock)
            {
                for (var i = 0; i < nonce.Length; i++)
                {
                    nonce[i] = chars[_random.Next(0, chars.Length)];
                }
            }

            return new string(nonce);
        }

        public static string GetTimestamp()
        {
            return GetTimestamp(DateTime.UtcNow);
        }

        public static string GetTimestamp(DateTime dateTime)
        {
            var timestamp = ToUnixTime(dateTime);
            return timestamp.ToString();
        }

        private static long ToUnixTime(DateTime dateTime)
        {
            var timeSpan = dateTime - new DateTime(1970, 1, 1);
            var timestamp = (long)timeSpan.TotalSeconds;

            return timestamp;
        }

        public static string UrlEncodeRelaxed(string value)
        {
            var escaped = Uri.EscapeDataString(value);

            escaped = escaped.Replace("(", PercentEncode("("))
                             .Replace(")", PercentEncode(")"));

            return escaped;
        }

        private static string PercentEncode(string s)
        {
            var bytes = Encoding.UTF8.GetBytes(s);
            var sb = new StringBuilder();
            foreach (var b in bytes)
            {
                if ((b > 7 && b < 11) || b == 13)
                {
                    sb.Append(string.Format("%0{0:X}", b));
                }
                else
                {
                    sb.Append(string.Format("%{0:X}", b));
                }
            }

            return sb.ToString();
        }

        public static string UrlEncodeStrict(string value)
        {
            var original = value;
            var ret = original.OfType<char>().Where(
                c => !Unreserved.OfType<char>().Contains(c) && c != '%').Aggregate(
                    value, (current, c) => current.Replace(
                          c.ToString(), PercentEncode(c.ToString())));

            return ret.Replace("%%", "%25%");
        }

        public static string NormalizeRequestParameters(WebParameterCollection parameters)
        {
            var copy = SortParametersExcludingSignature(parameters);
            var concatenated = Concatenate(copy, "=", "&");
            return concatenated;
        }

        private static string Concatenate(ICollection<WebParameter> collection, string separator, string spacer)
        {
            var sb = new StringBuilder();

            var total = collection.Count;
            var count = 0;

            foreach (var item in collection)
            {
                sb.Append(item.Name);
                sb.Append(separator);
                sb.Append(item.Value);

                count++;
                if (count < total)
                {
                    sb.Append(spacer);
                }
            }

            return sb.ToString();
        }

        public static WebParameterCollection SortParametersExcludingSignature(WebParameterCollection parameters)
        {
            var copy = new WebParameterCollection(parameters);
            var exclusions = copy.Where(n => EqualsIgnoreCase(n.Name, "oauth_signature"));

            copy.RemoveAll(exclusions);

            foreach (var parameter in copy)
            {
                parameter.Value = UrlEncodeStrict(parameter.Value);
            }

            copy.Sort((x, y) => x.Name.Equals(y.Name) ? x.Value.CompareTo(y.Value) : x.Name.CompareTo(y.Name));
            return copy;
        }

        private static bool EqualsIgnoreCase(string left, string right)
        {
            return string.Compare(left, right, StringComparison.InvariantCultureIgnoreCase) == 0;
        }

        public static string ConstructRequestUrl(Uri url)
        {
            if (url == null)
            {
                throw new ArgumentNullException(nameof(url));
            }

            var sb = new StringBuilder();

            var requestUrl = string.Format("{0}://{1}", url.Scheme, url.Host);
            var qualified = string.Format(":{0}", url.Port);
            var basic = url.Scheme == "http" && url.Port == 80;
            var secure = url.Scheme == "https" && url.Port == 443;

            sb.Append(requestUrl);
            sb.Append(!basic && !secure ? qualified : "");
            sb.Append(url.AbsolutePath);

            return sb.ToString();
        }

        public static string ConcatenateRequestElements(string method, string url, WebParameterCollection parameters)
        {
            var sb = new StringBuilder();

            var requestMethod = string.Concat(method.ToUpper(), "&");
            var requestUrl = string.Concat(UrlEncodeRelaxed(ConstructRequestUrl(new Uri(url))), "&");
            var requestParameters = UrlEncodeRelaxed(NormalizeRequestParameters(parameters));

            sb.Append(requestMethod);
            sb.Append(requestUrl);
            sb.Append(requestParameters);

            return sb.ToString();
        }

        public static string GetSignature(OAuthSignatureMethod signatureMethod,
                                          string signatureBase,
                                          string consumerSecret)
        {
            return GetSignature(signatureMethod, OAuthSignatureTreatment.Escaped, signatureBase, consumerSecret, null);
        }

        public static string GetSignature(OAuthSignatureMethod signatureMethod,
                                          OAuthSignatureTreatment signatureTreatment,
                                          string signatureBase,
                                          string consumerSecret)
        {
            return GetSignature(signatureMethod, signatureTreatment, signatureBase, consumerSecret, null);
        }

        public static string GetSignature(OAuthSignatureMethod signatureMethod,
                                          string signatureBase,
                                          string consumerSecret,
                                          string? tokenSecret)
        {
            return GetSignature(signatureMethod, OAuthSignatureTreatment.Escaped, consumerSecret, tokenSecret);
        }

        public static string GetSignature(OAuthSignatureMethod signatureMethod,
                                          OAuthSignatureTreatment signatureTreatment,
                                          string signatureBase,
                                          string consumerSecret,
                                          string? tokenSecret)
        {
            if (string.IsNullOrEmpty(tokenSecret))
            {
                tokenSecret = string.Empty;
            }

            consumerSecret = UrlEncodeRelaxed(consumerSecret);
            tokenSecret = UrlEncodeRelaxed(tokenSecret);

            string signature;
            switch (signatureMethod)
            {
                case OAuthSignatureMethod.HmacSha1:
                    {
                        var key = string.Concat(consumerSecret, "&", tokenSecret);
                        var crypto = new HMACSHA1();

                        crypto.Key = _encoding.GetBytes(key);
                        signature = HashWith(signatureBase, crypto);

                        break;
                    }

                default:
                    throw new NotImplementedException("Only HMAC-SHA1 is currently supported.");
            }

            var result = signatureTreatment == OAuthSignatureTreatment.Escaped
                       ? UrlEncodeRelaxed(signature)
                       : signature;

            return result;
        }

        private static string HashWith(string input, HashAlgorithm algorithm)
        {
            var data = Encoding.UTF8.GetBytes(input);
            var hash = algorithm.ComputeHash(data);
            return Convert.ToBase64String(hash);
        }
    }
}
