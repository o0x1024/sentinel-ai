#!/usr/bin/env node
/**
 * Generate update manifest for Tauri auto-updater
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const VERSION = process.env.npm_package_version || '0.1.0';
const REPO = process.env.GITHUB_REPOSITORY || 'owner/sentinel-ai';
const RELEASE_NOTES = process.env.RELEASE_NOTES || 'See the release notes for details.';
const UPLOAD_DIR = process.env.UPLOAD_DIR || '';

// Platform configurations
const PLATFORMS = {
  'darwin-x86_64': {
    url: `https://github.com/${REPO}/releases/download/v${VERSION}/sentinel-ai-${VERSION}-x86_64.app.tar.gz`,
    sigFile: `sentinel-ai-${VERSION}-x86_64.app.tar.gz.sig`
  },
  'darwin-aarch64': {
    url: `https://github.com/${REPO}/releases/download/v${VERSION}/sentinel-ai-${VERSION}-aarch64.app.tar.gz`,
    sigFile: `sentinel-ai-${VERSION}-aarch64.app.tar.gz.sig`
  },
  'windows-x86_64': {
    url: `https://github.com/${REPO}/releases/download/v${VERSION}/sentinel-ai-${VERSION}-x64-setup.nsis.zip`,
    sigFile: `sentinel-ai-${VERSION}-x64-setup.nsis.zip.sig`
  }
};

function readSignature(sigFile) {
  const paths = [
    path.join(UPLOAD_DIR, sigFile),
    path.join('dist/update-manifests', sigFile),
    sigFile
  ];

  for (const sigPath of paths) {
    if (sigPath && fs.existsSync(sigPath)) {
      return fs.readFileSync(sigPath, 'utf8').trim();
    }
  }

  console.warn(`Signature file not found: ${sigFile}`);
  return '';
}

function generateManifest() {
  const manifest = {
    version: VERSION,
    notes: RELEASE_NOTES,
    pub_date: new Date().toISOString(),
    platforms: {}
  };

  for (const [platform, config] of Object.entries(PLATFORMS)) {
    const signature = readSignature(config.sigFile);
    manifest.platforms[platform] = {
      signature: signature,
      url: config.url
    };
  }

  return manifest;
}

function main() {
  const outputDir = 'dist/update-manifests';

  // Create output directory
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  const manifest = generateManifest();

  // Write latest.json (main update manifest)
  const latestPath = path.join(outputDir, 'latest.json');
  fs.writeFileSync(latestPath, JSON.stringify(manifest, null, 2));
  console.log(`Generated: ${latestPath}`);

  // Write platform-specific manifests
  for (const platform of Object.keys(PLATFORMS)) {
    const platformManifest = {
      version: manifest.version,
      notes: manifest.notes,
      pub_date: manifest.pub_date,
      platforms: {
        [platform]: manifest.platforms[platform]
      }
    };

    const platformPath = path.join(outputDir, `${platform}.json`);
    fs.writeFileSync(platformPath, JSON.stringify(platformManifest, null, 2));
    console.log(`Generated: ${platformPath}`);
  }

  console.log('Update manifest generation completed.');
}

main();

