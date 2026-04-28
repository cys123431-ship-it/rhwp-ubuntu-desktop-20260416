import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const studioRoot = path.resolve(__dirname, '..');

function read(relativePath) {
  return fs.readFileSync(path.join(studioRoot, relativePath), 'utf8');
}

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(fullPath) : [fullPath];
  });
}

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    process.exitCode = 1;
  } else {
    console.log(`PASS: ${message}`);
  }
}

const shortcutSource = read('src/command/shortcut-map.ts');
const bindings = [...shortcutSource.matchAll(/(?:single|chord)\('([^']+)'\s*,\s*'([^']+)'/g)]
  .map((match) => ({ commandId: match[1], label: match[2] }));

const commandIds = new Set();
for (const file of walk(path.join(studioRoot, 'src/command/commands')).filter((f) => f.endsWith('.ts'))) {
  const source = fs.readFileSync(file, 'utf8');
  for (const match of source.matchAll(/id:\s*`view:zoom-\$\{pct\}`/g)) {
    void match;
    for (const pct of [50, 75, 100, 125, 150, 200, 300]) {
      commandIds.add(`view:zoom-${pct}`);
    }
  }
  for (const match of source.matchAll(/id:\s*'([^']+)'/g)) {
    commandIds.add(match[1]);
  }
  for (const match of source.matchAll(/stub\('([^']+)'/g)) {
    commandIds.add(match[1]);
  }
}

function hasBinding(commandId, label) {
  return bindings.some((binding) => binding.commandId === commandId && binding.label === label);
}

assert(hasBinding('format:font-size-increase', 'Ctrl+]'), 'font-size increase keeps Ctrl+] alias');
assert(hasBinding('format:font-size-increase', 'Alt+Shift+E'), 'font-size increase has Hancom Alt+Shift+E alias');
assert(hasBinding('format:font-size-decrease', 'Ctrl+['), 'font-size decrease keeps Ctrl+[ alias');
assert(hasBinding('format:font-size-decrease', 'Alt+Shift+R'), 'font-size decrease has Hancom Alt+Shift+R alias');

const orphanBindings = bindings.filter((binding) => !commandIds.has(binding.commandId));
assert(
  orphanBindings.length === 0,
  `all shortcut bindings resolve to registered commands (${orphanBindings.map((b) => `${b.label}->${b.commandId}`).join(', ')})`,
);

const duplicateStrokes = new Map();
for (const binding of bindings) {
  const key = binding.label;
  const existing = duplicateStrokes.get(key) ?? [];
  existing.push(binding.commandId);
  duplicateStrokes.set(key, existing);
}
const unsafeDuplicates = [...duplicateStrokes.entries()]
  .filter(([, commands]) => new Set(commands).size > 1)
  .filter(([label]) => !['Ctrl+Enter'].includes(label));
assert(
  unsafeDuplicates.length === 0,
  `no unexpected shortcut label collisions (${unsafeDuplicates.map(([label, commands]) => `${label}:${commands.join('/')}`).join(', ')})`,
);
