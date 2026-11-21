import { test } from 'vitest';
import assert from 'node:assert/strict';
import { characterLabel, expectedCharacter, parseCharacterKey } from '../src/character-key.ts';

test('character keys accept the empty realm WoW Forever reports', () => {
  const realmless = JSON.stringify(['WoW Forever', '', 'Fedrath']);
  assert.deepEqual(parseCharacterKey(realmless), {
    game: 'WoW Forever',
    realm: '',
    name: 'Fedrath',
  });
  assert.equal(characterLabel(realmless), 'Fedrath');
  assert.equal(characterLabel(JSON.stringify(['WoW Forever', 'Realm', 'Mage'])), 'Mage · Realm');
  assert.throws(() => parseCharacterKey(JSON.stringify(['WoW Forever', 'Realm', ''])));
});

test('expected characters come only from a valid key', () => {
  assert.deepEqual(expectedCharacter(JSON.stringify(['WoW Forever', '', 'Fedrath'])), {
    expectedCharacter: { realm: '', name: 'Fedrath' },
  });
  assert.deepEqual(expectedCharacter(undefined), {});
  assert.deepEqual(expectedCharacter('not json'), {});
});
