#!/bin/bash

# Deploy script for DataFusion Fiddle Lambda
# Usage: ./scripts/deploy.sh [dev|staging|prod] [aws-profile]

set -e

ENVIRONMENT=${1:-dev}
AWS_PROFILE=${2:-default}

echo "🚀 Deploying DataFusion Fiddle to $ENVIRONMENT environment"
echo "📦 Using AWS Profile: $AWS_PROFILE"

# Export AWS profile
export AWS_PROFILE=$AWS_PROFILE

# Build the Lambda function
echo "🔨 Building Lambda function..."
cargo lambda build --release --bin lambda

# Create SAM build directory
mkdir -p .aws-sam/build/DataFusionFunction
cp target/lambda/lambda/bootstrap .aws-sam/build/DataFusionFunction/

# Copy parquet files if they exist
if [ -d "api/parquet" ]; then
    echo "📁 Copying parquet files..."
    cp -r api/parquet .aws-sam/build/DataFusionFunction/
fi

# Deploy based on environment
case $ENVIRONMENT in
    dev)
        echo "🌱 Deploying to development..."
        sam deploy --config-env default --no-confirm-changeset
        ;;
    staging)
        echo "🎯 Deploying to staging..."
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
        echo "❌ Invalid environment: $ENVIRONMENT"
        echo "Usage: $0 [dev|staging|prod] [aws-profile]"
        exit 1
        ;;
esac

echo "✅ Deployment complete!"
echo "🔗 Check the CloudFormation stack in AWS Console for the API URL"