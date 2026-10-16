// Independent ECMAScript/V8 oracle; no npm packages or network access.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const executable = process.argv[2];
assert.ok(executable, 'Usage: node verify_jcs_ecmascript.mjs <built canonicalize example>');
const oracle = value => {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(oracle).join(',') + ']';
  return '{' + Object.keys(value).sort().map(k => JSON.stringify(k) + ':' + oracle(value[k])).join(',') + '}';
};
function verify(input) {
  const output = spawnSync(executable, [], { input, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024, timeout: 30000 });
  assert.equal(output.error, undefined);
  assert.equal(output.status, 0, output.stderr);
  assert.equal(output.stdout, oracle(JSON.parse(input)));
}
// Deterministic raw IEEE754 values span exponents, signs, denormals and ordinary values.
let state = 0x6a09e667f3bcc909n;
const mask = (1n << 64n) - 1n;
const values = [-0, Number.MIN_VALUE, -Number.MIN_VALUE, Number.MAX_VALUE, -Number.MAX_VALUE,
  1e-7, 1e-6, 1e20, 1e21, 1e23, 333333333.33333329];
const bits = Buffer.alloc(8);
for (let i = 0; i < 4096; i++) {
  state ^= (state << 13n) & mask; state ^= state >> 7n; state ^= (state << 17n) & mask; state &= mask;
  bits.writeBigUInt64LE(state);
  const value = bits.readDoubleLE();
  if (Number.isFinite(value)) values.push(value);
}
// Batches keep the same production 8192 element input budget.
for (let i = 0; i < values.length; i += 1024) {
  verify('{"\ue000":1,"😀":2,"a":' + JSON.stringify(values.slice(i, i + 1024)) + '}');
}
for (const vector of JSON.parse(readFileSync(path.join(root, 'fixtures/canonical-vectors.json'), 'utf8'))) verify(vector.input);
verify('{"\\r":"CR","1":"one","\\u0080":"ctrl","ö":"latin","€":"euro","😀":"emoji","דּ":"hebrew"}');
console.log(`ECMAScript/V8 differential passed: ${values.length} finite IEEE754 values, frozen vectors and UTF16 object order`);
// Check the actual emitted ECMA-262 regexes, without claiming full Schema validation.
const schema = JSON.parse(readFileSync(path.join(root, 'schemas/engineering.schema.json'), 'utf8'));
const identifier = schema.$defs.Identifier;
const patterns = [identifier.pattern, ...identifier.allOf.map(s => s.pattern)].map(p => new RegExp(p, 'u'));
for (const text of [' ', '\u3000', ' \u00a0 ', 'user\u0085suffix', '\u009f']) {
  assert.equal(patterns.every(p => p.test(text)), false, JSON.stringify(text));
}
for (const text of ['ARCH-1', '订单领域', 'unicode-é']) assert.ok(patterns.every(p => p.test(text)));
assert.match(identifier['x-semanticConstraint'], /UTF-8 byte/);
console.log('Identifier Schema regex constraints passed (whitespace/C1); UTF8 byte budget remains explicit semantic validation');
