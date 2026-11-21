import { test } from 'vitest';
import assert from 'node:assert/strict';
import { characterLabel, parseCharacterKey } from '../src/character-key.ts';

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
