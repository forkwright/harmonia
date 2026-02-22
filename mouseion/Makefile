# Mouseion — Unified media manager
# Common development commands

.PHONY: build test run clean restore format lint docker-up docker-down migrate help

SOLUTION = Mouseion.sln
HOST = src/Mouseion.Host/Mouseion.Host.csproj

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the solution
	dotnet build $(SOLUTION)

test: ## Run all tests
	dotnet test $(SOLUTION) --no-build --verbosity normal

test-fast: ## Run tests without build (assumes already built)
	dotnet test $(SOLUTION) --no-build --no-restore -v minimal

run: ## Run the application
	dotnet run --project $(HOST)

run-dev: ## Run with Development environment
	ASPNETCORE_ENVIRONMENT=Development dotnet run --project $(HOST)

watch: ## Run with hot reload
	dotnet watch run --project $(HOST)

restore: ## Restore NuGet packages
	dotnet restore $(SOLUTION)

clean: ## Clean build artifacts
	dotnet clean $(SOLUTION)
	find . -type d -name bin -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name obj -exec rm -rf {} + 2>/dev/null || true

format: ## Format code with dotnet format
	dotnet format $(SOLUTION)

format-check: ## Check code formatting (CI mode)
	dotnet format $(SOLUTION) --verify-no-changes

lint: ## Run analyzers
	dotnet build $(SOLUTION) /p:TreatWarningsAsErrors=true

migrate: ## Show current migration status
	@echo "Migrations are applied automatically on startup."
	@echo "Latest migration files:"
	@ls -1 src/Mouseion.Core/Datastore/Migration/*.cs | sort | tail -5

docker-up: ## Start local dev environment
	docker compose -f docker-compose.dev.yml up -d

docker-down: ## Stop local dev environment
	docker compose -f docker-compose.dev.yml down

docker-logs: ## View docker logs
	docker compose -f docker-compose.dev.yml logs -f

swagger: ## Open Swagger UI (assumes running on :5000)
	@echo "http://localhost:5000/swagger"
	@command -v xdg-open && xdg-open http://localhost:5000/swagger || true

count: ## Count lines of code
	@echo "Source:"
	@find src -name "*.cs" -not -path "*/obj/*" -not -path "*/bin/*" | xargs wc -l | tail -1
	@echo "Tests:"
	@find tests -name "*.cs" -not -path "*/obj/*" -not -path "*/bin/*" | xargs wc -l | tail -1

coverage: ## Run tests with coverage report
	dotnet test $(SOLUTION) --collect:"XPlat Code Coverage" --results-directory ./coverage
	@echo "Coverage reports in ./coverage/"
