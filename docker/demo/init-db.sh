#!/bin/bash
set -e

# Note: 
# - mono database is created automatically by PostgreSQL container via POSTGRES_DB environment variable
# - orion-server uses the same mono database (no separate database needed)
# - campsite database is managed by MySQL service, not PostgreSQL
echo "PostgreSQL initialization complete"
echo "Note: mono database is created automatically by PostgreSQL container"
echo "Note: Campsite database is managed by MySQL service, not PostgreSQL"

