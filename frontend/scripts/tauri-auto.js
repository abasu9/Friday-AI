#!/usr/bin/env node
/**
 * Auto-detect GPU and run Tauri with appropriate features
 */

const { execSync } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { ensureSidecar } = require('./prepare-tauri-sidecar');
const DEV_PORT = 3118;

function parseEnvFile(filePath) {
  if (!fs.existsSync(filePath)) {
    return {};
  }

  const parsed = {};
  const content = fs.readFileSync(filePath, 'utf8');

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }

    const match = line.match(/^([^=]+)=(.*)$/);
    if (!match) {
      continue;
    }

    const key = match[1].trim();
    let value = match[2].trim();

    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    parsed[key] = value;
  }

  return parsed;
}

function loadLocalEnv() {
  const cwd = process.cwd();
  return {
    ...parseEnvFile(path.join(cwd, '.env')),
    ...parseEnvFile(path.join(cwd, '.env.local')),
  };
}

function getListeningPids(port) {
  try {
    const output = execSync(`lsof -tiTCP:${port} -sTCP:LISTEN`, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'ignore'],
    });
    return output
      .split(/\r?\n/)
      .map((value) => value.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

function getProcessCommand(pid) {
  try {
    return execSync(`ps -o command= -p ${pid}`, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return '';
  }
}

function isNextDevProcess(commandLine, port) {
  if (!commandLine) {
    return false;
  }

  return (
    commandLine.includes('next dev') ||
    (commandLine.includes('next/dist/bin/next') &&
      (commandLine.includes(`-p ${port}`) || commandLine.includes(` ${port}`)))
  );
}

function sleep(ms) {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    // Busy wait is acceptable here because this is a short-lived dev launcher.
  }
}

function waitForPortToFree(port, timeoutMs = 3000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (getListeningPids(port).length === 0) {
      return true;
    }
    sleep(100);
  }

  return getListeningPids(port).length === 0;
}

function ensureFrontendPortAvailable(port) {
  const pids = getListeningPids(port);
  if (pids.length === 0) {
    return;
  }

  const owners = pids.map((pid) => ({
    pid,
    command: getProcessCommand(pid),
  }));
  const nextOwners = owners.filter(({ command }) => isNextDevProcess(command, port));

  if (nextOwners.length === owners.length) {
    console.log(`🧹 Found existing Next dev server on port ${port}; stopping it before launch.`);
    for (const { pid } of nextOwners) {
      try {
        process.kill(Number(pid), 'SIGTERM');
      } catch {
        // Ignore races where the process exits before we signal it.
      }
    }

    if (!waitForPortToFree(port)) {
      console.log(`⚠️  Next dev server on port ${port} did not exit after SIGTERM; forcing it down.`);
      for (const { pid } of nextOwners) {
        try {
          process.kill(Number(pid), 'SIGKILL');
        } catch {
          // Ignore races where the process exits before we signal it.
        }
      }
    }

    if (waitForPortToFree(port)) {
      return;
    }
  }

  const ownerDetails = owners
    .map(({ pid, command }) => `PID ${pid}: ${command || 'unknown process'}`)
    .join('\n');
  console.error(`❌ Port ${port} is already in use.\n${ownerDetails}`);
  console.error('Close the existing process or change the frontend dev port before running tauri:dev.');
  process.exit(1);
}

const localEnv = loadLocalEnv();
const env = {
  ...localEnv,
  ...process.env,
};

// Get the command (dev or build)
const command = process.argv[2];
if (!command || !['dev', 'build'].includes(command)) {
  console.error('Usage: node tauri-auto.js [dev|build]');
  process.exit(1);
}

const requestedFeature = process.argv[3];

// Detect GPU feature
let feature = '';

// CLI override takes precedence, then environment variable, then auto-detection.
if (requestedFeature) {
  feature = requestedFeature;
  console.log(`🔧 Using forced GPU feature from CLI: ${feature}`);
} else if (env.TAURI_GPU_FEATURE) {
  feature = env.TAURI_GPU_FEATURE;
  console.log(`🔧 Using forced GPU feature from environment: ${feature}`);
} else {
  try {
    const result = execSync('node scripts/auto-detect-gpu.js', {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'inherit']
    });
    feature = result.trim();
  } catch (err) {
    // If detection fails, continue with no features
  }
}

console.log(''); // Empty line for spacing

// Platform-specific environment variables
const platform = os.platform();

if (command === 'dev') {
  ensureFrontendPortAvailable(DEV_PORT);
}

if (platform === 'linux' && feature === 'cuda') {
  console.log('🐧 Linux/CUDA detected: Setting CMAKE flags for NVIDIA GPU');
  env.CMAKE_CUDA_ARCHITECTURES = '75';
  env.CMAKE_CUDA_STANDARD = '17';
  env.CMAKE_POSITION_INDEPENDENT_CODE = 'ON';
}

try {
  ensureSidecar(command, feature || 'none');
  console.log('');
} catch (err) {
  console.error(`❌ Failed to prepare llama-helper sidecar: ${err.message || err}`);
  process.exit(1);
}

// Build the tauri command
let tauriCmd = `tauri ${command}`;
if (feature && feature !== 'none') {
  tauriCmd += ` -- --features ${feature}`;
  console.log(`🚀 Running: tauri ${command} with features: ${feature}`);
} else {
  console.log(`🚀 Running: tauri ${command} (CPU-only mode)`);
}
console.log('');

// Execute the command
try {
  execSync(tauriCmd, { stdio: 'inherit', env });
} catch (err) {
  process.exit(err.status || 1);
}
