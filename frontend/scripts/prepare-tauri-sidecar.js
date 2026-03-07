#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const frontendDir = path.resolve(__dirname, '..');
const workspaceRoot = path.resolve(frontendDir, '..');
const helperDir = path.join(workspaceRoot, 'llama-helper');
const binariesDir = path.join(frontendDir, 'src-tauri', 'binaries');

function run(cmd, args, cwd) {
  execFileSync(cmd, args, {
    cwd,
    stdio: 'inherit'
  });
}

function detectTargetTriple() {
  const output = execFileSync('rustc', ['-vV'], {
    cwd: workspaceRoot,
    encoding: 'utf8'
  });
  const hostLine = output
    .split('\n')
    .find((line) => line.startsWith('host: '));

  if (!hostLine) {
    throw new Error('Unable to detect Rust target triple from `rustc -vV`.');
  }

  return hostLine.replace('host: ', '').trim();
}

function mapHelperFeature(feature) {
  if (!feature || feature === 'none') {
    return null;
  }

  if (feature === 'coreml') {
    console.log('Note: llama-helper does not support CoreML, using Metal instead.');
    return 'metal';
  }

  if (feature === 'openblas' || feature === 'hipblas') {
    console.log(`Note: llama-helper does not support ${feature}; falling back to CPU.`);
    return null;
  }

  return feature;
}

function getPaths(mode, targetTriple) {
  const isWindows = process.platform === 'win32';
  const buildProfile = mode === 'build' ? 'release' : 'debug';
  const baseBinary = isWindows ? 'llama-helper.exe' : 'llama-helper';
  const sidecarBinary = isWindows
    ? `llama-helper-${targetTriple}.exe`
    : `llama-helper-${targetTriple}`;

  return {
    sourceBinary: path.join(workspaceRoot, 'target', buildProfile, baseBinary),
    sidecarBinary,
    sidecarPath: path.join(binariesDir, sidecarBinary),
    stampPath: path.join(binariesDir, `.llama-helper-${targetTriple}.json`)
  };
}

function readStamp(stampPath) {
  try {
    return JSON.parse(fs.readFileSync(stampPath, 'utf8'));
  } catch {
    return null;
  }
}

function writeStamp(stampPath, stamp) {
  fs.writeFileSync(stampPath, `${JSON.stringify(stamp, null, 2)}\n`);
}

function cleanOldSidecars(currentName) {
  if (!fs.existsSync(binariesDir)) {
    return;
  }

  for (const entry of fs.readdirSync(binariesDir)) {
    if (!entry.startsWith('llama-helper')) {
      continue;
    }

    if (entry === currentName) {
      continue;
    }

    fs.rmSync(path.join(binariesDir, entry), { force: true });
  }
}

function ensureSidecar(mode, feature) {
  if (!fs.existsSync(helperDir)) {
    throw new Error(`llama-helper directory not found at ${helperDir}`);
  }

  fs.mkdirSync(binariesDir, { recursive: true });

  const targetTriple = detectTargetTriple();
  const helperFeature = mapHelperFeature(feature);
  const desiredStamp = {
    mode,
    feature: helperFeature || 'cpu',
    targetTriple
  };

  const { sourceBinary, sidecarBinary, sidecarPath, stampPath } = getPaths(mode, targetTriple);
  const currentStamp = readStamp(stampPath);

  let shouldBuild = !fs.existsSync(sourceBinary);

  if (!shouldBuild && currentStamp) {
    shouldBuild =
      currentStamp.mode !== desiredStamp.mode ||
      currentStamp.feature !== desiredStamp.feature ||
      currentStamp.targetTriple !== desiredStamp.targetTriple;
  }

  if (!shouldBuild && !currentStamp && !fs.existsSync(sidecarPath)) {
    shouldBuild = true;
  }

  if (shouldBuild) {
    const args = ['build'];
    if (mode === 'build') {
      args.push('--release');
    }
    if (helperFeature) {
      args.push('--features', helperFeature);
    }

    console.log('');
    console.log(`Preparing llama-helper sidecar (${mode})...`);
    console.log(`Target triple: ${targetTriple}`);
    console.log(`llama-helper feature: ${helperFeature || 'cpu'}`);
    run('cargo', args, helperDir);
  }

  if (!fs.existsSync(sourceBinary)) {
    throw new Error(`llama-helper binary not found after build at ${sourceBinary}`);
  }

  const shouldCopy =
    shouldBuild ||
    !fs.existsSync(sidecarPath) ||
    fs.statSync(sourceBinary).mtimeMs > fs.statSync(sidecarPath).mtimeMs;

  if (shouldCopy) {
    cleanOldSidecars(sidecarBinary);
    fs.copyFileSync(sourceBinary, sidecarPath);
    fs.chmodSync(sidecarPath, 0o755);
    console.log(`Copied llama-helper sidecar to ${path.relative(frontendDir, sidecarPath)}`);
  }

  writeStamp(stampPath, desiredStamp);

  return {
    feature: helperFeature,
    sidecarPath,
    targetTriple
  };
}

module.exports = {
  ensureSidecar
};

if (require.main === module) {
  const mode = process.argv[2];
  const feature = process.argv[3] || null;

  if (!mode || !['dev', 'build'].includes(mode)) {
    console.error('Usage: node scripts/prepare-tauri-sidecar.js [dev|build] [feature]');
    process.exit(1);
  }

  try {
    ensureSidecar(mode, feature);
  } catch (error) {
    console.error(error.message || error);
    process.exit(1);
  }
}
