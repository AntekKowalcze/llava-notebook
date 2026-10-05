/** Anchored, so `"junk a@b.co junk"` is rejected. Same rule on every form. */
export const EMAIL_PATTERN =
  /^[\p{L}\p{N}!#$%&'*+/=?^_`{|}~-]+(?:\.[\p{L}\p{N}!#$%&'*+/=?^_`{|}~-]+)*@(?:[\p{L}\p{N}](?:[\p{L}\p{N}-]*[\p{L}\p{N}])?\.)+[\p{L}\p{N}](?:[\p{L}\p{N}-]*[\p{L}\p{N}])?$/u;

export function isValidEmail(email: string): boolean {
  return EMAIL_PATTERN.test(email.trim());
}
