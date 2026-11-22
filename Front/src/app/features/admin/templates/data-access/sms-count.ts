/** Characters of the GSM 03.38 basic alphabet (one unit each). */
const GSM_BASIC =
  '@£$¥èéùìòÇ\nØø\rÅåΔ_ΦΓΛΩΠΨΣΘΞÆæßÉ !"#¤%&\'()*+,-./0123456789:;<=>?' +
  '¡ABCDEFGHIJKLMNOPQRSTUVWXYZÄÖÑÜ§¿abcdefghijklmnopqrstuvwxyzäöñüà';
/** Characters of the GSM extension table (two units each). */
const GSM_EXTENDED = '^{}\\[~]|€\f';

export interface SmsCount {
  /** Units used: characters, extension characters counting twice in GSM-7. */
  length: number;
  unicode: boolean;
  segments: number;
}

/**
 * Length and number of SMS of a text (F007, F016): GSM-7 160 characters for one SMS, 153 per
 * part beyond; Unicode (an "ê", an emoji…) 70, then 67 per part.
 */
export function smsCount(text: string): SmsCount {
  const characters = Array.from(text);
  const unicode = characters.some((c) => !GSM_BASIC.includes(c) && !GSM_EXTENDED.includes(c));
  const length = unicode
    ? characters.reduce((total, c) => total + ((c.codePointAt(0) ?? 0) > 0xffff ? 2 : 1), 0)
    : characters.reduce((total, c) => total + (GSM_EXTENDED.includes(c) ? 2 : 1), 0);
  const [single, part] = unicode ? [70, 67] : [160, 153];
  const segments = length === 0 ? 0 : length <= single ? 1 : Math.ceil(length / part);
  return { length, unicode, segments };
}
