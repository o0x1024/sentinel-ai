import https from 'https';
import http from 'http';
import { createWriteStream, mkdirSync, chmodSync, copyFileSync } from 'fs';
import { join } from 'path';
import { execSync } from 'child_process';

const __dirname = new URL('.', import.meta.url).pathname;
const projectRoot = join(__dirname, '..');
const binDir = join(projectRoot, 'src-tauri', 'bin');
const SIDECAR_BASE_NAME = 'sentinel-agent-browser';

// agent-browser release versions
const AGENT_BROWSER_VERSION = 'v0.21.2';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64';
  }
  if (platform === 'linux') {
    return arch === 'arm64' ? 'linux-arm64' : 'linux-x64';
  }
  if (platform === 'win32') {
    return 'windows-x64';
  }

  throw new Error(`Unsupported platform: ${platform}`);
}

function getTauriTargetTriple() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  }

  if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  }

  if (platform === 'win32') {
    return 'x86_64-pc-windows-msvc';
  }

  throw new Error(`Unsupported platform for Tauri target triple: ${platform}`);
}

function downloadFile(url, outputPath) {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith('https') ? https : http;
    const file = createWriteStream(outputPath);

    protocol.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        downloadFile(response.headers.location, outputPath)
          .then(resolve)
          .catch(reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      response.pipe(file);

      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      file.destroy();
      reject(err);
    });
  });
}

async function main() {
  try {
    const platform = getPlatform();
    const targetTriple = getTauriTargetTriple();
    const binaryName = platform.startsWith('windows')
      ? `${SIDECAR_BASE_NAME}.exe`
      : SIDECAR_BASE_NAME;
    const outputPath = join(binDir, binaryName);
    const tauriBinaryName = platform.startsWith('windows')
      ? `${SIDECAR_BASE_NAME}-${targetTriple}.exe`
      : `${SIDECAR_BASE_NAME}-${targetTriple}`;
    const tauriOutputPath = join(binDir, tauriBinaryName);

    console.log(`Downloading agent-browser ${AGENT_BROWSER_VERSION} for ${platform}...`);

    // Create bin directory
    mkdirSync(binDir, { recursive: true });

    // Download from GitHub releases
    const downloadUrl = `https://github.com/vercel-labs/agent-browser/releases/download/${AGENT_BROWSER_VERSION}/agent-browser-${platform}`;

    await downloadFile(downloadUrl, outputPath);

    // Create Tauri sidecar target-triple filename
    copyFileSync(outputPath, tauriOutputPath);

    // Make executable (not needed on Windows)
    if (!platform.startsWith('windows')) {
      chmodSync(outputPath, 0o755);
      chmodSync(tauriOutputPath, 0o755);
    }

    console.log(`✓ agent-browser downloaded to ${outputPath}`);
    console.log(`✓ tauri sidecar binary created at ${tauriOutputPath}`);

    // Verify the binary works
    try {
      const version = execSync(`"${outputPath}" --version`).toString().trim();
      console.log(`✓ agent-browser version: ${version}`);
    } catch (err) {
      console.warn('⚠ Could not verify binary, but download completed');
    }
  } catch (error) {
    console.error('✗ Failed to download agent-browser:', error.message);
    process.exit(1);
  }
}

main();
