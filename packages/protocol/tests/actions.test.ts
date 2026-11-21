import { test } from 'vitest';
import assert from 'node:assert/strict';
import { actionSchema, macroBodyError } from '../src/game-actions.ts';
test('action schemas reject invalid coordinates, control bytes and overlong UTF-8 macro text', () => {
  for (const body of [
    '#showtooltip\n/cast [@mouseover,help,nodead][] Frostbolt',
    '/run print(1)',
    '/script x=1',
    '/click Button',
    '/say hello |cffff0000red|r',
    '/tar Hogger\n/sc Frostbolt',
  ]) {
    assert.equal(macroBodyError(body), undefined);
  }
  for (const body of ['   ', '/cast Frostbolt\x01', 'x'.repeat(256)]) {
    assert.ok(macroBodyError(body));
  }
  assert.throws(() =>
    actionSchema.parse({
      kind: 'set_waypoint',
      args: { mapId: 1, x: 101, y: 50, label: 'NPC', source: 'user' },
    }),
  );
  assert.throws(() =>
    actionSchema.parse({
      kind: 'create_macro',
      args: { name: 'Macro', body: '/cast ' + 'é'.repeat(125) },
    }),
  );
  assert.throws(() =>
    actionSchema.parse({ kind: 'create_macro', args: { name: '   ', body: '/cast Frostbolt' } }),
  );
  assert.throws(() => actionSchema.parse({ kind: 'move', args: {} }));
});

test('Polestar guide actions require each goal type’s fields and bound the guide', () => {
  const goal = { type: 'accept', questId: 786, questTitle: 'Thwarting Kolkar Aggression' };
  const guide = (steps: unknown[], extra = {}) =>
    actionSchema.safeParse({ kind: 'polestar_guide', args: { title: 'Kolkar', steps, ...extra } });
  const parsed = actionSchema.parse({
    kind: 'polestar_guide',
    args: { title: ' Kolkar ', steps: [{ goals: [goal] }] },
  });
  assert.deepEqual(parsed.args, { title: 'Kolkar', steps: [{ goals: [goal] }], open: true });
  assert.equal(guide([{ goals: [{ ...goal, mapId: 1411, x: 51.9, y: 43.5 }] }]).success, true);
  assert.equal(guide([{ goals: [{ type: 'goto', mapId: 1411, x: 50, y: 50 }] }]).success, true);
  assert.equal(guide([{ goals: [{ type: 'note', text: 'Watch for patrols.' }] }]).success, true);
  // Required fields per type, paired coordinates, unknown types and fields.
  assert.equal(guide([{ goals: [{ type: 'accept', questId: 786 }] }]).success, false);
  assert.equal(guide([{ goals: [{ type: 'goto', mapId: 1411, x: 50 }] }]).success, false);
  assert.equal(guide([{ goals: [{ ...goal, mapId: 1411 }] }]).success, false);
  assert.equal(guide([{ goals: [{ ...goal, x: 101, y: 1, mapId: 1 }] }]).success, false);
  assert.equal(guide([{ goals: [{ type: 'teleport' }] }]).success, false);
  assert.equal(guide([{ goals: [{ ...goal, script: 'x' }] }]).success, false);
  // Size bounds and ids.
  assert.equal(guide([]).success, false);
  assert.equal(guide([{ goals: [] }]).success, false);
  assert.equal(guide([{ goals: Array(9).fill(goal) }]).success, false);
  assert.equal(guide(Array(31).fill({ goals: [goal] })).success, false);
  assert.equal(guide([{ goals: [goal] }], { id: 'g1700000000' }).success, true);
  assert.equal(guide([{ goals: [goal] }], { id: '../Bad' }).success, false);
});
