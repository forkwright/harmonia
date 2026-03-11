// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Filters;
using Mouseion.Api.Common;

namespace Mouseion.Api.Validation;

/// <summary>
/// Action filter that validates request models using FluentValidation
/// and returns ProblemDetails on validation failure
/// </summary>
public class ValidationFilter : IAsyncActionFilter
{
    private readonly IServiceProvider _serviceProvider;

    public ValidationFilter(IServiceProvider serviceProvider)
    {
        _serviceProvider = serviceProvider;
    }

    public async Task OnActionExecutionAsync(ActionExecutingContext context, ActionExecutionDelegate next)
    {
        var errors = new Dictionary<string, string[]>();

        foreach (var argument in context.ActionArguments.Values)
        {
            if (argument == null) continue;

            var argumentType = argument.GetType();
            var validatorType = typeof(IValidator<>).MakeGenericType(argumentType);
            var validator = _serviceProvider.GetService(validatorType) as IValidator;

            if (validator == null) continue;

            var validationContext = new ValidationContext<object>(argument);
            var result = await validator.ValidateAsync(validationContext, context.HttpContext.RequestAborted);

            if (!result.IsValid)
            {
                foreach (var error in result.Errors)
                {
                    var key = error.PropertyName;
                    if (errors.ContainsKey(key))
                    {
                        errors[key] = errors[key].Append(error.ErrorMessage).ToArray();
                    }
                    else
                    {
                        errors[key] = new[] { error.ErrorMessage };
                    }
                }
            }
        }

        if (errors.Any())
        {
            var problemDetails = new ApiProblemDetails
            {
                Status = 422,
                Title = "Validation Failed",
                Detail = "One or more validation errors occurred.",
                Type = "https://tools.ietf.org/html/rfc4918#section-11.2",
                Errors = errors
            };

            context.Result = new UnprocessableEntityObjectResult(problemDetails);
            return;
        }

        await next();
    }
}
