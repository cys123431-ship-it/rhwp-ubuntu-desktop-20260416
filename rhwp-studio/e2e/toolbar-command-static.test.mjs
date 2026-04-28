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

const indexHtml = read('index.html');
const commandSources = walk(path.join(studioRoot, 'src/command/commands'))
  .filter((file) => file.endsWith('.ts'))
  .map((file) => fs.readFileSync(file, 'utf8'))
  .join('\n');

const visibleToolbarCommands = [
  'edit:cut',
  'edit:copy',
  'edit:paste',
  'edit:format-copy',
  'view:ctrl-mark',
  'view:para-mark',
  'view:grid-settings',
  'format:char-shape',
  'format:para-shape',
  'format:toggle-numbering',
  'format:level-increase',
  'format:level-decrease',
  'table:create',
  'insert:shape',
  'insert:image',
  'format:object-properties',
  'insert:symbols',
  'insert:hyperlink',
  'page:header-create',
  'page:footer-create',
  'insert:footnote',
  'insert:endnote',
  'edit:find',
];

const coreToolbarCommands = visibleToolbarCommands.filter((cmd) => ![
  'insert:hyperlink',
  'insert:endnote',
].includes(cmd));

function commandExists(commandId) {
  return commandSources.includes(`id: '${commandId}'`)
    || commandSources.includes(`stub('${commandId}'`)
    || commandSources.includes(`id: \`${commandId}\``);
}

function isStubbed(commandId) {
  if (commandSources.includes(`stub('${commandId}'`)) return true;
  const escaped = commandId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const commandBlock = new RegExp(`id:\\s*'${escaped}'[\\s\\S]*?\\n\\s*},`, 'm').exec(commandSources)?.[0] ?? '';
  return /canExecute:\s*\(\)\s*=>\s*false/.test(commandBlock);
}

for (const commandId of visibleToolbarCommands) {
  assert(indexHtml.includes(`data-cmd="${commandId}"`), `toolbar button is wired to ${commandId}`);
  assert(commandExists(commandId), `toolbar command exists: ${commandId}`);
}

for (const commandId of coreToolbarCommands) {
  assert(!isStubbed(commandId), `core toolbar command is not stubbed: ${commandId}`);
}

assert(isStubbed('insert:hyperlink'), 'visible hyperlink button remains explicitly tracked as a stub');
assert(isStubbed('insert:endnote'), 'visible endnote button remains explicitly tracked as a stub');
