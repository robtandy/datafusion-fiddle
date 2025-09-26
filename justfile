# DataFusion Fiddle Justfile
# https://github.com/casey/just

# List available commands
default:
    @just --list

# Install required tools
install:
    echo "Installing Cargo Lambda..."
    brew tap cargo-lambda/cargo-lambda || true
    brew install cargo-lambda || true
    echo "Installing AWS SAM CLI..."
    brew install aws-sam-cli || true
    echo "Tools installed successfully!"

# Build Lambda function (development mode)
build-lambda:
    echo "Building Lambda function (dev mode)..."
    cargo lambda build --bin lambda --arm64 --output-format zip --include api/parquet
    echo "Lambda build complete!"

# Build Lambda function (release mode)
build-lambda-release:
    echo "Building Lambda function (release mode)..."
    cargo lambda build --release --bin lambda --arm64
    echo "Lambda release build complete!"

# Build with Cargo Lambda (includes parquet files directly)
build-sam: build-lambda
    echo "Cargo Lambda build complete with parquet files included"

# Deploy to AWS (requires AWS credentials configured)
deploy: build-sam
    echo "Deploying to AWS..."
    sam deploy --guided

# Deploy to specific environment
deploy-env env='dev': build-sam
    #!/usr/bin/env bash
    case {{env}} in
        dev)
            sam deploy --config-env default --no-confirm-changeset
            ;;
        staging)
            sam deploy --config-env staging
            ;;
        prod)
            echo "🚨 Deploying to production..."
            read -p "Are you sure you want to deploy to production? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                sam deploy --config-env prod
            else
                echo "❌ Production deployment cancelled"
                exit 1
            fi
            ;;
        *)
            echo "❌ Invalid environment: {{env}}"
            echo "Usage: just deploy-env [dev|staging|prod]"
            exit 1
            ;;
    esac

# Deploy with specific AWS profile
deploy-with-profile env='dev' profile='default':
    echo "🚀 Deploying to {{env}} with AWS profile {{profile}}"
    AWS_PROFILE={{profile}} just deploy-env {{env}}

# Run Lambda locally with SAM
local port='3001': build-sam
    echo "Starting local Lambda API on port {{port}}..."
    sam local start-api --port {{port}} --warm-containers EAGER

# Test Lambda locally with Cargo Lambda
local-cargo:
    echo "Starting local Lambda with Cargo Lambda..."
    cargo lambda watch --bin lambda

# Clean build artifacts
clean:
    echo "Cleaning build artifacts..."
    cargo clean
    rm -rf .aws-sam
    echo "Clean complete!"

# Run frontend development server
frontend-dev:
    echo "Starting frontend development server..."
    pnpm dev

# Build frontend for production
frontend-build:
    echo "Building frontend..."
    pnpm build

# Full local development (frontend + backend in parallel)
dev:
    #!/usr/bin/env bash
    echo "Starting full development environment..."
    # Start backend in background
    just local &
    BACKEND_PID=$!
    # Give backend time to start
    sleep 2
    # Start frontend
    just frontend-dev &
    FRONTEND_PID=$!
    # Wait for Ctrl+C and cleanup
    trap "kill $BACKEND_PID $FRONTEND_PID; exit" INT
    wait

# Test the Lambda function locally
test-local api_url='http://localhost:3001':
    echo "Testing local Lambda at {{api_url}}..."
    curl -X POST {{api_url}}/api/sql \
        -H "Content-Type: application/json" \
        -d '{"distributed": false, "stmts": ["SELECT 1 as test"]}'

# Test with custom SQL
test-sql sql='SELECT 1 as test' api_url='http://localhost:3001':
    echo "Executing SQL: {{sql}}"
    curl -X POST {{api_url}}/api/sql \
        -H "Content-Type: application/json" \
        -d '{"distributed": false, "stmts": ["{{sql}}"]}'

# View Lambda logs
logs stack='datafusion-fiddle':
    sam logs -n DataFusionFunction --stack-name {{stack}} --tail

# Validate SAM template
validate:
    echo "Validating SAM template..."
    sam validate
    echo "✅ Template is valid!"

# Build both Lambda and frontend
build-all: build-lambda frontend-build
    echo "✅ All builds complete!"

# Run linting and type checking
check:
    echo "Running frontend lint..."
    pnpm lint
    echo "Checking Rust code..."
    cargo check --all-targets

# Format code
format:
    echo "Formatting Rust code..."
    cargo fmt
    echo "Formatting frontend code..."
    pnpm format || echo "Note: Add 'format' script to package.json for frontend formatting"

# Show deployment info
info:
    echo "📦 Project: DataFusion Fiddle"
    echo "🎯 Lambda Binary: target/lambda/lambda/bootstrap"
    echo "🔧 Frontend: http://localhost:5173 (dev)"
    echo "🚀 API Local: http://localhost:3001/api/sql"
    echo ""
    echo "AWS Configuration:"
    aws sts get-caller-identity 2>/dev/null || echo "⚠️  AWS credentials not configured"
    echo ""
    echo "Installed Tools:"
    which cargo-lambda >/dev/null 2>&1 && echo "✅ Cargo Lambda installed" || echo "❌ Cargo Lambda not installed"
    which sam >/dev/null 2>&1 && echo "✅ SAM CLI installed" || echo "❌ SAM CLI not installed"
    which docker >/dev/null 2>&1 && echo "✅ Docker installed" || echo "❌ Docker not installed"

# Quick start for new developers
quickstart: install
    echo "🚀 Setting up DataFusion Fiddle..."
    cp .env.example .env 2>/dev/null || true
    pnpm install
    just build-lambda
    echo ""
    echo "✅ Setup complete! Run 'just dev' to start development"

# Interactive deployment menu
deploy-interactive:
    #!/usr/bin/env bash
    echo "🚀 DataFusion Fiddle Deployment"
    echo "================================"
    echo "Select environment:"
    echo "1) Development"
    echo "2) Staging"
    echo "3) Production"
    echo "4) Cancel"
    read -p "Choice [1-4]: " choice
    case $choice in
        1) just deploy-env dev ;;
        2) just deploy-env staging ;;
        3) just deploy-env prod ;;
        4) echo "Cancelled" ;;
        *) echo "Invalid choice" ;;
    esac

# Run a specific Cargo Lambda command
cargo-lambda *args:
    cargo lambda {{args}}

# Run a specific SAM command
sam *args:
    sam {{args}}