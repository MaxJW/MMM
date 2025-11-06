#!/bin/bash

# Update script for MMM dashboard application
# Pulls latest changes, installs dependencies, builds, and restarts services

git pull && \
docker compose build && \
docker compose up -d && \
sudo systemctl restart dashboard-kiosk
