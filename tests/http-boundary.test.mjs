import test from 'node:test';
import assert from 'node:assert/strict';
import { evaluateHttp } from './http-contract.mjs';

test('Live HTTP integrity and compatibility contract', async (t) => {
  const evidence = await evaluateHttp(process.cwd());
  for (const check of [...evidence.holdout.checks, ...evidence.anchor.checks]) {
    await t.test(check.name, () => assert.equal(check.pass, true));
  }
});
