#!/bin/bash
# Test runner script for Python orchestrator unit tests
#
# Usage:
#   ./run_tests.sh              # Run all tests
#   ./run_tests.sh submission   # Run submission tests only
#   ./run_tests.sh retry        # Run retry tests only
#   ./run_tests.sh journal      # Run journal tests only
#   ./run_tests.sh coverage     # Run with coverage report

set -e

cd "$(dirname "$0")"

echo "==================================="
echo "Python Orchestrator Unit Tests"
echo "==================================="
echo

if [ "$1" == "coverage" ]; then
    echo "Running tests with coverage report..."
    uv run pytest --cov=. --cov-report=term-missing --cov-report=html
    echo
    echo "Coverage report generated in htmlcov/index.html"
    
elif [ "$1" == "submission" ]; then
    echo "Running submission tests..."
    uv run pytest tests/test_submission.py -v
    
elif [ "$1" == "retry" ]; then
    echo "Running retry tests..."
    uv run pytest tests/test_retry.py -v
    
elif [ "$1" == "journal" ]; then
    echo "Running journal tests..."
    uv run pytest tests/test_journal.py -v
    
elif [ "$1" == "quick" ]; then
    echo "Running quick tests (no coverage)..."
    uv run pytest -v --tb=short
    
elif [ "$1" == "debug" ]; then
    echo "Running tests with detailed output..."
    uv run pytest -vv --tb=long --log-cli-level=DEBUG
    
else
    echo "Running all tests..."
    uv run pytest -v
fi

echo
echo "==================================="
echo "Tests Complete!"
echo "==================================="

