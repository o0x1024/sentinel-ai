#!/bin/bash
# Check if creating a self-signed developer certificate on macOS stops the "damaged" error.
# Usually, many actions do this:
security create-keychain -p "build" build.keychain
security default-keychain -s build.keychain
security unlock-keychain -p "build" build.keychain
# Create a self-signed cert
# ... too complicated
