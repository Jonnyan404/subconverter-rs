#!/bin/bash

set -e # Exit immediately if a command exits with a non-zero status.

# Check for required commands
if ! command -v curl > /dev/null 2>&1; then
    echo "Error: curl is not installed. Please install it (e.g., sudo apt install curl)."
    exit 1
fi
if ! command -v jq > /dev/null 2>&1; then
    echo "Error: jq is not installed. Please install it (e.g., sudo apt install jq)."
    exit 1
fi

REPO="lonelam/subconverter-rs"

echo "Detecting OS and architecture..."
OS=$(uname -s)
ARCH=$(uname -m)

# Map OS and Arch to GitHub release asset naming conventions
case "$OS" in
    Linux) GITHUB_OS="linux" ;;
    Darwin) GITHUB_OS="macos" ;;
    *) echo "Error: Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64) GITHUB_ARCH="amd64" ;; # GitHub uses amd64 for x86_64
    aarch64) GITHUB_ARCH="aarch64" ;;
    armv7l) GITHUB_ARCH="armv7" ;; # Assuming armv7l maps to armv7
    *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Determine the expected asset suffix based on OS
if [ "$GITHUB_OS" == "linux" ] || [ "$GITHUB_OS" == "macos" ]; then
    ASSET_SUFFIX=".tar.gz"
    EXTRACT_CMD="tar -xzf"
else
    # Add logic for other OS like Windows (.7z or .zip) if needed later
    echo "Error: Script currently only supports Linux and macOS downloads."
    exit 1
fi

echo "Fetching latest release information from $REPO for $GITHUB_OS-$GITHUB_ARCH..."
API_URL="https://api.github.com/repos/$REPO/releases/latest"

# Construct the expected asset name pattern
ASSET_PATTERN="subconverter-${GITHUB_OS}-${GITHUB_ARCH}"

echo "Looking for asset matching pattern: *${ASSET_PATTERN}*${ASSET_SUFFIX}"

# Use jq to find the download URL
DOWNLOAD_URL=$(curl -s "$API_URL" | jq -r --arg pattern "$ASSET_PATTERN" --arg suffix "$ASSET_SUFFIX" '.assets[] | select(.name | contains($pattern) and endswith($suffix)).browser_download_url')

if [ -z "$DOWNLOAD_URL" ]; then
  echo "Error: Could not automatically find the download URL for $GITHUB_OS-$GITHUB_ARCH."
  echo "Please check the releases page manually: https://github.com/$REPO/releases/latest"
  exit 1
fi

echo "Found download URL: $DOWNLOAD_URL"

# Store the current directory
ORIGINAL_DIR=$(pwd)

# Use the actual filename from the URL for the archive
ARCHIVE_NAME=$(basename "$DOWNLOAD_URL")

echo "Downloading $ARCHIVE_NAME to $(pwd)..."
# Clean up previous archive if it exists
rm -f "$ARCHIVE_NAME"
curl -L -o "$ARCHIVE_NAME" "$DOWNLOAD_URL"
if [ $? -ne 0 ]; then
    echo "Error: Download failed."
    exit 1
fi

# Define the expected subdirectory name after extraction
EXTRACTED_SUBDIR="subconverter"

# Clean up previous extracted directory if it exists
echo "Removing existing directory '$EXTRACTED_SUBDIR' if present..."
rm -rf "./$EXTRACTED_SUBDIR"

echo "Extracting $ARCHIVE_NAME..."
$EXTRACT_CMD "$ARCHIVE_NAME"
if [ $? -ne 0 ]; then
    echo "Error: Extraction failed."
    rm "$ARCHIVE_NAME" # Clean up downloaded archive
    exit 1
fi

echo "Cleaning up downloaded archive..."
rm "$ARCHIVE_NAME"

# Check if the expected subdirectory exists
if [ ! -d "./$EXTRACTED_SUBDIR" ]; then
    echo "Error: Expected subdirectory '$EXTRACTED_SUBDIR' not found after extraction."
    echo "Contents of $(pwd):"
    ls -l
    exit 1
fi

echo "Changing working directory to '$EXTRACTED_SUBDIR'..."
cd "./$EXTRACTED_SUBDIR"

# Find the executable in the current directory (which is now the extracted subdir)
EXECUTABLE=$(find . -maxdepth 1 -type f \( -name 'subconverter-rs' -o -name 'subconverter' \))

if [ -z "$EXECUTABLE" ]; then
    echo "Error: Could not find the executable (subconverter-rs or subconverter) in $(pwd)."
    echo "Contents of $(pwd):"
    ls -l
    cd "$ORIGINAL_DIR" # Go back to original directory
    exit 1
fi

# Get just the filename (remove leading ./)
EXECUTABLE_NAME=$(basename "$EXECUTABLE")

echo "Making '$EXECUTABLE_NAME' executable..."
chmod +x "$EXECUTABLE_NAME"

echo "Running '$EXECUTABLE_NAME' from $(pwd)..."
echo "Access the API at http://127.0.0.1:25500 (Press Ctrl+C to stop)"
# Run in the foreground from the current directory
./"$EXECUTABLE_NAME"

# The script will exit here when the server is stopped (e.g., with Ctrl+C)
echo "'$EXECUTABLE_NAME' stopped."

# Go back to the original directory
cd "$ORIGINAL_DIR"

echo "Setup and execution finished." 