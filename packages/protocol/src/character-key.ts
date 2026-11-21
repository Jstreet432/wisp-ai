import { z } from 'zod';
// [game, realm, name]; the realm is empty in WoW Forever, which has no realms.
const identity = z.tuple([z.string().min(1), z.string(), z.string().min(1)]);
export function parseCharacterKey(key: string): { game: string; realm: string; name: string } {
  const [game, realm, name] = identity.parse(JSON.parse(key));
  return { game, realm, name };
}
export function characterLabel(key: string, separator = ' · '): string {
  try {
    const { name, realm } = parseCharacterKey(key);
    return realm ? name + separator + realm : name;
  } catch {
    return key;
  }
}
