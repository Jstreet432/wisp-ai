import { z } from 'zod';
// [game, realm, name]; the realm is empty in WoW Forever, which has no realms.
const identity = z.tuple([z.string().min(1), z.string(), z.string().min(1)]);
export function parseCharacterKey(key: string): { game: string; realm: string; name: string } {
  const [game, realm, name] = identity.parse(JSON.parse(key));
  return { game, realm, name };
}
/**
 * The character an action must run as, for the addon to double-check. Empty
 * when the turn had no identity; the transport session still pins the character.
 */
export function expectedCharacter(key: string | undefined): {
  expectedCharacter?: { realm: string; name: string };
} {
  if (!key) return {};
  try {
    const { realm, name } = parseCharacterKey(key);
    return { expectedCharacter: { realm, name } };
  } catch {
    return {};
  }
}
export function characterLabel(key: string, separator = ' · '): string {
  try {
    const { name, realm } = parseCharacterKey(key);
    return realm ? name + separator + realm : name;
  } catch {
    return key;
  }
}
