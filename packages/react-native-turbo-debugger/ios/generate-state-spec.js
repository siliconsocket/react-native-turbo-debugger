'use strict';

// Generate into this Debug-only pod, outside the app-wide Codegen registry.
const {execFileSync} = require('node:child_process');
const {mkdtempSync, mkdirSync, copyFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const {dirname, join} = require('node:path');

const reactNative = dirname(require.resolve('react-native/package.json'));
const codegen = dirname(require.resolve('@react-native/codegen/package.json', {paths: [reactNative]}));
const name = 'RNTurboDebuggerStateSpec';
const work = mkdtempSync(join(tmpdir(), 'rntd-codegen-'));
try {
  const schema = join(work, 'schema.json');
  const output = join(work, 'output');
  execFileSync(process.execPath, [
    join(codegen, 'lib/cli/combine/combine-js-to-schema-cli.js'), schema, join(__dirname, '../specs'),
  ]);
  execFileSync(process.execPath, [
    join(reactNative, 'scripts/generate-specs-cli.js'), '--platform', 'ios',
    '--schemaPath', schema, '--outputDir', output, '--libraryName', name, '--libraryType', 'modules',
  ]);
  const generated = join(__dirname, 'generated');
  mkdirSync(generated, {recursive: true});
  for (const file of [name + '.h', name + '-generated.mm']) {
    copyFileSync(join(output, name, file), join(generated, file));
  }
} finally {
  rmSync(work, {recursive: true, force: true});
}
