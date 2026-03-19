import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { cp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const workspaceDir = path.join(repoRoot, '.artifacts', 'agent-browser-upstream');
const cloneDir = path.join(workspaceDir, 'repo');
const runtimeDir = path.join(workspaceDir, 'runtime');
const bundleDir = path.join(repoRoot, 'src-tauri', 'agent-browser-bundle');
const repoUrl =
  process.env.AGENT_BROWSER_REPO_URL ?? 'https://github.com/vercel-labs/agent-browser.git';
const repoRef = process.env.AGENT_BROWSER_REF ?? 'main';

function commandName(name) {
  return process.platform === 'win32' ? `${name}.cmd` : name;
}

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? repoRoot,
      env: { ...process.env, ...(options.env ?? {}) },
      stdio: 'inherit',
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`Command failed (${code}): ${command} ${args.join(' ')}`));
    });
  });
}

function readStdout(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: process.env,
      stdio: ['ignore', 'pipe', 'inherit'],
    });

    let output = '';
    child.stdout.on('data', (chunk) => {
      output += String(chunk);
    });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(output.trim());
        return;
      }
      reject(new Error(`Command failed (${code}): ${command} ${args.join(' ')}`));
    });
  });
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, 'utf8'));
}

function detectPackageManager(pkg, sourceDir) {
  const packageManager = pkg.packageManager ?? '';

  if (packageManager.startsWith('pnpm@') || existsSync(path.join(sourceDir, 'pnpm-lock.yaml'))) {
    return {
      install: [commandName('corepack'), ['pnpm', 'install', '--frozen-lockfile']],
      build: [commandName('corepack'), ['pnpm', 'build']],
      needsCorepack: true,
    };
  }

  if (existsSync(path.join(sourceDir, 'package-lock.json'))) {
    return {
      install: [commandName('npm'), ['ci']],
      build: [commandName('npm'), ['run', 'build']],
      needsCorepack: false,
    };
  }

  if (packageManager.startsWith('yarn@') || existsSync(path.join(sourceDir, 'yarn.lock'))) {
    return {
      install: [commandName('corepack'), ['yarn', 'install', '--frozen-lockfile']],
      build: [commandName('corepack'), ['yarn', 'build']],
      needsCorepack: true,
    };
  }

  return {
    install: [commandName('npm'), ['install']],
    build: [commandName('npm'), ['run', 'build']],
    needsCorepack: false,
  };
}

async function copyRuntimePackage() {
  const installedRoot = path.join(runtimeDir, 'node_modules', 'agent-browser');
  const installedPackageJson = path.join(installedRoot, 'package.json');
  const installedDist = path.join(installedRoot, 'dist');
  const installedNodeModules = path.join(runtimeDir, 'node_modules');

  if (!existsSync(installedPackageJson) || !existsSync(installedDist)) {
    throw new Error(`agent-browser runtime package missing in ${installedRoot}`);
  }

  await rm(bundleDir, { recursive: true, force: true });
  await mkdir(path.join(bundleDir, 'node_modules'), { recursive: true });

  await cp(installedDist, path.join(bundleDir, 'dist'), { recursive: true });
  await cp(installedPackageJson, path.join(bundleDir, 'package.json'));

  const dependencyEntries = await readdir(installedNodeModules);
  for (const entry of dependencyEntries) {
    if (entry === 'agent-browser') {
      continue;
    }
    await cp(path.join(installedNodeModules, entry), path.join(bundleDir, 'node_modules', entry), {
      recursive: true,
    });
  }
}

async function main() {
  console.log(`Preparing latest agent-browser from ${repoUrl}#${repoRef}`);

  await rm(workspaceDir, { recursive: true, force: true });
  await rm(bundleDir, { recursive: true, force: true });
  await mkdir(workspaceDir, { recursive: true });

  await run(commandName('git'), ['clone', '--depth', '1', '--branch', repoRef, repoUrl, cloneDir]);

  const pkg = await readJson(path.join(cloneDir, 'package.json'));
  const packageManager = detectPackageManager(pkg, cloneDir);

  if (packageManager.needsCorepack) {
    await run(commandName('corepack'), ['enable'], { cwd: cloneDir });
  }

  await run(packageManager.install[0], packageManager.install[1], { cwd: cloneDir });
  await run(packageManager.build[0], packageManager.build[1], { cwd: cloneDir });

  await rm(runtimeDir, { recursive: true, force: true });
  await mkdir(runtimeDir, { recursive: true });
  await writeFile(
    path.join(runtimeDir, 'package.json'),
    JSON.stringify({ private: true, name: 'agent-browser-runtime' }, null, 2) + os.EOL,
  );

  await run(
    commandName('npm'),
    ['install', '--omit=dev', '--ignore-scripts', '--no-save', cloneDir],
    { cwd: runtimeDir },
  );

  await copyRuntimePackage();

  const commit = await readStdout(commandName('git'), ['rev-parse', 'HEAD'], cloneDir);
  await writeFile(
    path.join(bundleDir, 'UPSTREAM_COMMIT'),
    `repo=${repoUrl}\nref=${repoRef}\ncommit=${commit}\n`,
  );

  console.log(`Prepared agent-browser bundle at ${bundleDir}`);
  console.log(`Using upstream commit ${commit}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
