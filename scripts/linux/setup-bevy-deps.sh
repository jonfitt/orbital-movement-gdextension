#!/usr/bin/env bash
# Install apt packages needed to build the Bevy visualizer on Debian/Ubuntu.
set -euo pipefail

if ! command -v apt-get >/dev/null 2>&1; then
	echo "error: apt-get is required (Debian/Ubuntu)." >&2
	exit 1
fi

sudo apt-get update
sudo apt-get install -y \
	build-essential \
	pkg-config \
	libasound2-dev \
	libudev-dev \
	libxkbcommon-dev \
	libwayland-dev \
	libx11-dev \
	libxcursor-dev \
	libxi-dev \
	libxrandr-dev \
	libvulkan-dev

echo "Bevy build dependencies installed."
