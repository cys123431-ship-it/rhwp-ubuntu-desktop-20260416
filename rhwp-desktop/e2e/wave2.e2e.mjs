import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, cp, mkdtemp, rm } from 'node:fs/promises';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { after, afterEach, before, beforeEach, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { Builder, Capabilities } from 'selenium-webdriver';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const isWindows = process.platform === 'win32';
const installedBinary = process.env.RHWP_E2E_APP
  ?? (process.platform === 'win32' ? 'C:\\Program Files\\rhwp\\rhwp.exe' : '/usr/bin/rhwp');
const tauriDriverUrl = process.env.RHWP_E2E_DRIVER_URL ?? 'http://127.0.0.1:4444';
const tauriDriverHost = process.env.RHWP_E2E_DRIVER_HOST ?? '127.0.0.1';
const tauriDriverPort = Number(process.env.RHWP_E2E_DRIVER_PORT ?? '4444');
const tauriDriverBin = process.env.RHWP_E2E_TAURI_DRIVER ?? 'tauri-driver';
const defaultAppPattern = /default app|기본 앱/i;
const settingsPattern = /settings|설정/i;

let tauriDriverProcess;
let tempRoot = '';
const spawnedInstances = new Set();

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
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

async function runCleanupProcess(command, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: 'ignore', windowsHide: true });
    child.once('error', reject);
    child.once('exit', () => resolve());
  });
}

async function terminateInstalledProcesses() {
  try {
    if (isWindows) {
      const escapedPath = installedBinary.replace(/'/g, "''");
      await runCleanupProcess('pwsh', [
        '-NoLogo',
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        `
          $path = '${escapedPath}'
          Get-CimInstance Win32_Process |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -ieq $path } |
            ForEach-Object {
              try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop } catch {}
            }
        `,
      ]);
    } else {
      await runCleanupProcess('pkill', ['-f', installedBinary]).catch(() => {});
    }
  } catch {
    // Best-effort cleanup for CI isolation.
  }

  await delay(500);
}

async function createTempCopy(relativeSourcePath, targetName) {
  const sourcePath = path.resolve(repoRoot, relativeSourcePath);
  const targetPath = path.join(tempRoot, targetName);
  await cp(sourcePath, targetPath);
  return targetPath;
}

async function openApp(args = []) {
  const capabilities = new Capabilities();
  capabilities.setBrowserName('wry');
  capabilities.set('tauri:options', { application: installedBinary, args });

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

async function launchAdditionalInstance(args = []) {
  const child = spawn(installedBinary, args, {
    stdio: 'ignore',
    windowsHide: true,
  });
  spawnedInstances.add(child);

  await Promise.race([
    once(child, 'exit'),
    delay(15000),
  ]);
}

before(async () => {
  await access(installedBinary);
  tempRoot = await mkdtemp(path.join(os.tmpdir(), 'rhwp-wave2-e2e-'));
  tauriDriverProcess = spawn(tauriDriverBin, [], {
    stdio: 'inherit',
    env: process.env,
    windowsHide: true,
  });
  await waitForPort(tauriDriverHost, tauriDriverPort, 30000);
}, { timeout: 120000 });

beforeEach(async () => {
  await terminateInstalledProcesses();
}, { timeout: 30000 });

afterEach(async () => {
  await terminateInstalledProcesses();
  for (const child of [...spawnedInstances]) {
    if (!child.killed && child.exitCode === null) {
      child.kill();
      await Promise.race([
        once(child, 'exit'),
        delay(5000),
      ]).catch(() => {});
    }
    spawnedInstances.delete(child);
  }
}, { timeout: 30000 });

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

test('shows the default-app banner and handles platform-specific registration flow', async () => {
  const driver = await openApp();

  try {
    if (isWindows) {
      const initialState = await waitForState(
        driver,
        (state) =>
          state.session.associationStatus?.platform === 'windows'
          && (
            (state.banner.visible && state.banner.actions.length > 0)
            || state.session.associationStatus?.isDefault === true
          ),
        15000,
        'windows file association state',
      );

      assert.equal(initialState.session.associationStatus?.platform, 'windows');

      if (initialState.banner.visible) {
        assert.match(initialState.banner.text, defaultAppPattern);
        assert.equal(await callHook(driver, 'clickBannerAction'), true);

        const updatedState = await waitForState(
          driver,
          (state) => state.banner.visible && settingsPattern.test(state.banner.text),
          15000,
          'windows default apps settings banner',
        );

        assert.equal(updatedState.session.associationStatus?.isDefault, false);
        assert.match(updatedState.banner.text, settingsPattern);
      } else {
        assert.equal(initialState.session.associationStatus?.isDefault, true);
      }
    } else {
      const initialState = await waitForState(
        driver,
        (state) => state.banner.visible && state.banner.actions.length > 0,
        15000,
        'default app banner',
      );

      assert.match(initialState.banner.text, defaultAppPattern);
      assert.equal(await callHook(driver, 'clickBannerAction'), true);

      const updatedState = await waitForState(
        driver,
        (state) => state.session.associationStatus?.isDefault === true,
        15000,
        'updated file association status',
      );

      assert.equal(updatedState.session.associationStatus?.isDefault, true);
      assert.equal(updatedState.banner.visible, false);
    }
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 60000 });

test('opens the Wave 2 representative HWPX sample as editable-safe', async () => {
  const driver = await openApp([
    path.resolve(repoRoot, 'samples', 'tac-img-02.hwpx'),
  ]);

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

  let driver = await openApp([workingCopy]);
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

  driver = await openApp([workingCopy]);

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

  driver = await openApp([workingCopy]);

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
  const driver = await openApp([hwpSample, hwpxSample]);

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

test('routes a second launch into a new window via single-instance handoff', async () => {
  const firstSample = await createTempCopy(
    path.join('samples', 're-01-hangul-only.hwp'),
    'handoff-first.hwp',
  );
  const secondSample = path.resolve(repoRoot, 'compatibility-corpus', 'fixtures', 'basic-shape.hwpx');
  const driver = await openApp([firstSample]);

  try {
    await waitForState(
      driver,
      (state) => state.session.hasDocument && state.session.fileName === path.basename(firstSample),
      20000,
      'initial handoff window load',
    );

    await launchAdditionalInstance([secondSample]);

    const handles = await waitForWindowCount(driver, 2, 30000);
    const seenFiles = new Set();

    for (const handle of handles) {
      await driver.switchTo().window(handle);
      const state = await waitForState(
        driver,
        (candidate) => candidate.ready === true && candidate.session.hasDocument,
        10000,
        'handoff window readiness',
      );
      seenFiles.add(state.session.fileName);
    }

    assert.deepEqual(
      seenFiles,
      new Set([path.basename(firstSample), path.basename(secondSample)]),
    );
  } finally {
    await driver.quit();
  }
}, { concurrency: false, timeout: 90000 });
