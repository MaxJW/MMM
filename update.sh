#!/bin/bash

# Update script for MMM dashboard application
# Pulls latest changes, installs dependencies, builds, and restarts services

git pull && \
bun i && \
bun run build && \
sudo systemctl daemon-reload && \
sudo systemctl restart dashboard-webapp && \
sudo systemctl restart dashboard-kiosk

