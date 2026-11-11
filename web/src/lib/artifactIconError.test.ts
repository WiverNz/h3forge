import assert from 'node:assert/strict';
import { test } from 'node:test';
import { artifactIconError } from './artifactIconError.ts';

test('preserves permission denials and structured resource errors', () => {
  assert.equal(artifactIconError('get_artifact_icon not allowed'), 'get_artifact_icon not allowed');
  assert.equal(artifactIconError({ code: 'asset_failed', message: 'Artifact.def missing' }), 'asset_failed: Artifact.def missing');
  assert.equal(artifactIconError(new Error('IPC failed')), 'IPC failed');
});
