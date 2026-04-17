import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, chmod, cp, mkdtemp, rm, writeFile } from 'node:fs/promises';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { after, before, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { Builder, Capabilities } from 'selenium-webdriver';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const installedBinary = process.env.RHWP_E2E_APP ?? '/usr/bin/rhwp';
const tauriDriverUrl = process.env.RHWP_E2E_DRIVER_URL ?? 'http://127.0.0.1:4444';
const tauriDriverHost = process.env.RHWP_E2E_DRIVER_HOST ?? '127.0.0.1';
const tauriDriverPort = Number(process.env.RHWP_E2E_DRIVER_PORT ?? '4444');
const tauriDriverBin = process.env.RHWP_E2E_TAURI_DRIVER ?? 'tauri-driver';

let tauriDriverProcess;
let tempRoot = '';

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\"'\"'`)}'`;
}

async function waitForPort(host, port, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    try {
      await new Promise((resolve, reject) => {
        const socket = net.createConnection({ host, port });
        socket.once('connect', () => {
          socket.destroy();
          resolve();
        });
        socket.once('error', (error) => {
          socket.destroy();
          reject(error);
        });
      });
      return;
    } catch {
      await delay(250);
    }
  }

  throw new Error(`Timed out waiting for tauri-driver on ${host}:${port}`);
}

async function waitFor(predicate, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  let lastError;

  while (Date.now() < deadline) {
    try {
      if (await predicate()) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await delay(200);
  }

  if (lastError) {
    throw lastError;
  }

  throw new Error(`Timed out waiting for ${label}`);
}

async function createLauncher(name, args = []) {
  const launcherPath = path.join(tempRoot, `${name}.sh`);
  const body = [
    '#!/usr/bin/env bash',
    'set -euo pipefail',
    `exec ${shellQuote(installedBinary)} "$@"${args.length ? ` ${args.map(shellQuote).join(' ')}` : ''}`,
    '',
  ].join('\n');

  await writeFile(launcherPath, body, 'utf8');
  await chmod(launcherPath, 0o755);
  return launcherPath;
}

async function createTempCopy(relativeSourcePath, targetName) {
  const sourcePath = path.resolve(repoRoot, relativeSourcePath);
  const targetPath = path.join(tempRoot, targetName);
  await cp(sourcePath, targetPath);
  return targetPath;
}

async function openApp(application) {
  const capabilities = new Capabilities();
  capabilities.setBrowserName('tauri');
  capabilities.set('tauri:options', { application });

  const driver = await new Builder()
    .usingServer(tauriDriverUrl)
    .withCapabilities(capabilities)
    .build();

  await waitFor(async () => {
    return driver.executeScript('return Boolean(window.__RHWP_E2E__?.isReady?.());');
  }, 30000, 'studio E2E bridge');

  return driver;
}

async function callHook(driver, method, ...args) {
  return driver.executeScript(
    `return window.__RHWP_E2E__.${method}.apply(window.__RHWP_E2E__, arguments);`,
    ...args,
  );
}

async function getState(driver) {
  return driver.executeScript('return window.__RHWP_E2E__.getRuntimeState();');
}

async function waitForState(driver, predicate, timeoutMs, label) {
  let state = null;
  await waitFor(async () => {
    state = await getState(driver);
    return predicate(state);
  }, timeoutMs, label);
  return state;
}

async function waitForWindowCount(driver, count, timeoutMs = 20000) {
  await waitFor(async () => {
    const handles = await driver.getAllWindowHandles();
    return handles.length === count;
  }, timeoutMs, `${count} desktop windows`);
  return driver.getAllWindowHandles();
}

before(async () => {
  await access(installedBinary);
  tempRoot = await mkdtemp(path.join(os.tmpdir(), 'rhwp-wave2-e2e-'));
  tauriDriverProcess = spawn(tauriDriverBin, [], {
    stdio: 'inherit',
    env: process.env,
  });
  await waitForPort(tauriDriverHost, tauriDriverPort, 30000);
}, { timeout: 120000 });

after(async () => {
  if (tauriDriverProcess && !tauriDriverProcess.killed) {
    tauriDriverProcess.kill('SIGTERM');
    await Promise.race([
      once(tauriDriverProcess, 'exit'),
      delay(5000),
    ]);
  }

  if (tempRoot) {
    await rm(tempRoot, { recursive: true, force: true });
  }
}, { timeout: 30000 });

test('shows first-run default-app banner and registers file associations', async () => {
  const launcher = await createLauncher('association');
  const driver = await openApp(launcher);

  try {
    const initialState = await waitForState(
      driver,
      (state) => state.banner.visible && state.banner.actions.length > 0,
      15000,
      'default app banner',
    );

    assert.match(initialState.banner.text, /default app/i);
    assert.equal(await callHook(driver, 'clickBannerAction'), true);

    const updatedState = await waitForState(
      driver,
      (state) => state.session.associationStatus?.isDefault === true,
      15000,
      'updated file association status',
    );

    assert.equal(updatedState.session.associationStatus?.isDefault, true);
    assert.equal(updatedState.banner.visible, false);
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 60000 });

test('opens the Wave 2 representative HWPX sample as editable-safe', async () => {
  const launcher = await createLauncher('tac-img-02', [
    path.resolve(repoRoot, 'samples', 'tac-img-02.hwpx'),
  ]);
  const driver = await openApp(launcher);

  try {
    const state = await waitForState(
      driver,
      (candidate) => candidate.session.hasDocument,
      20000,
      'representative Wave 2 sample load',
    );

    assert.equal(state.session.fileName, 'tac-img-02.hwpx');
    assert.equal(state.session.sourceFormat, 'hwpx');
    assert.equal(state.session.editMode, 'editable-safe');
    assert.equal(state.session.isProtected, false);
    assert.ok(state.pageCount > 0);
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 60000 });

test('restores and clears recovery snapshots for dirty editable documents', async () => {
  const workingCopy = await createTempCopy(
    path.join('samples', 're-01-hangul-only.hwp'),
    'wave2-recovery.hwp',
  );

  const firstLaunch = await createLauncher('recovery-first', [workingCopy]);
  let driver = await openApp(firstLaunch);
  let snapshotId = null;

  try {
    await waitForState(
      driver,
      (state) => state.session.hasDocument && state.session.filePath === workingCopy,
      20000,
      'initial recovery fixture load',
    );

    assert.equal(await callHook(driver, 'appendTextToParagraph', ' wave2-e2e'), true);
    const dirtyState = await waitForState(
      driver,
      (state) => state.session.dirty === true,
      10000,
      'dirty session state',
    );
    assert.equal(dirtyState.session.dirty, true);

    snapshotId = await callHook(driver, 'flushRecoverySnapshot');
    assert.ok(snapshotId);

    const snapshots = await callHook(driver, 'listRecoverySnapshots');
    assert.ok(snapshots.some((item) => item.id === snapshotId));
  } finally {
    await driver.quit();
  }

  const secondLaunch = await createLauncher('recovery-second', [workingCopy]);
  driver = await openApp(secondLaunch);

  try {
    const dialogState = await waitForState(
      driver,
      (state) => state.dialog.visible === true,
      15000,
      'recovery offer dialog',
    );
    assert.equal(dialogState.dialog.visible, true);

    assert.equal(await callHook(driver, 'acceptActiveDialog'), true);
    const recoveredState = await waitForState(
      driver,
      (state) => state.session.recoverySnapshotId === snapshotId,
      20000,
      'recovered session state',
    );

    assert.equal(recoveredState.session.fileName, path.basename(workingCopy));
    assert.equal(await callHook(driver, 'saveCurrentDocument'), true);

    const savedState = await waitForState(
      driver,
      (state) => state.session.dirty === false && state.session.recoverySnapshotId === null,
      15000,
      'saved recovery session',
    );
    assert.equal(savedState.session.dirty, false);

    const snapshotsAfterSave = await callHook(driver, 'listRecoverySnapshots');
    assert.ok(!snapshotsAfterSave.some((item) => item.id === snapshotId));
  } finally {
    await driver.quit();
  }

  const thirdLaunch = await createLauncher('recovery-third', [workingCopy]);
  driver = await openApp(thirdLaunch);

  try {
    const finalState = await waitForState(
      driver,
      (state) => state.session.hasDocument,
      15000,
      'final recovery verification load',
    );
    assert.equal(finalState.dialog.visible, false);
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 120000 });

test('opens one window per startup file when multiple documents are provided', async () => {
  const hwpSample = await createTempCopy(
    path.join('samples', 're-01-hangul-only.hwp'),
    'fanout-left.hwp',
  );
  const hwpxSample = path.resolve(repoRoot, 'compatibility-corpus', 'fixtures', 'basic-shape.hwpx');
  const launcher = await createLauncher('fanout', [hwpSample, hwpxSample]);
  const driver = await openApp(launcher);

  try {
    const handles = await waitForWindowCount(driver, 2, 30000);
    const seenFiles = new Set();

    for (const handle of handles) {
      await driver.switchTo().window(handle);
      const state = await waitForState(
        driver,
        (candidate) => candidate.ready === true,
        10000,
        'per-window bridge readiness',
      );
      seenFiles.add(state.session.fileName);
    }

    assert.deepEqual(
      seenFiles,
      new Set([path.basename(hwpSample), path.basename(hwpxSample)]),
    );
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 90000 });
