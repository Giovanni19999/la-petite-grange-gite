import {
  formatEuros,
  formatPercent,
  parseEuros,
  parsePercent,
} from '../../prices/data-access/price-math';
import { OptionUnit, Promotion, StayOption } from './offers.api';

/** A promotion as typed in the form (text, so that "10,5" can be typed). */
export interface PromotionDraft {
  nameFr: string;
  nameEn: string;
  kind: 'percent' | 'per_stay' | 'per_night';
  value: string;
  min: string;
  max: string;
  from: string;
  until: string;
  active: boolean;
}

/** An option as typed in the form. */
export interface OptionDraft {
  nameFr: string;
  nameEn: string;
  descriptionFr: string;
  descriptionEn: string;
  unit: OptionUnit;
  price: string;
  min: string;
  max: string;
  position: string;
  active: boolean;
}

export const EMPTY_PROMOTION: PromotionDraft = {
  nameFr: '',
  nameEn: '',
  kind: 'percent',
  value: '',
  min: '',
  max: '',
  from: '',
  until: '',
  active: true,
};

export const EMPTY_OPTION: OptionDraft = {
  nameFr: '',
  nameEn: '',
  descriptionFr: '',
  descriptionEn: '',
  unit: 'stay',
  price: '0',
  min: '',
  max: '',
  position: '0',
  active: true,
};

const nights = (value: number | null) => (value === null ? '' : String(value));

export function promotionDraft(promotion: Promotion): PromotionDraft {
  const discount = promotion.discount;
  return {
    nameFr: promotion.name.fr,
    nameEn: promotion.name.en,
    kind: discount.kind,
    value: discount.kind === 'percent' ? formatPercent(discount.bp) : formatEuros(discount.amount),
    min: nights(promotion.min_nights),
    max: nights(promotion.max_nights),
    from: promotion.valid_from ?? '',
    until: promotion.valid_until ?? '',
    active: promotion.active,
  };
}

export function optionDraft(option: StayOption): OptionDraft {
  return {
    nameFr: option.name.fr,
    nameEn: option.name.en,
    descriptionFr: option.description.fr,
    descriptionEn: option.description.en,
    unit: option.unit,
    price: formatEuros(option.price),
    min: nights(option.min_nights),
    max: nights(option.max_nights),
    position: String(option.position),
    active: option.active,
  };
}

/** Optional number of nights: empty is open, otherwise a whole number of at least 1. */
function bound(text: string): number | null | undefined {
  const value = text.trim();
  if (value === '') {
    return null;
  }
  return /^\d+$/.test(value) && Number(value) >= 1 ? Number(value) : undefined;
}

function boundErrors(min: number | null | undefined, max: number | null | undefined): string[] {
  if (min === undefined || max === undefined) {
    return ['Nuits minimum et maximum : un nombre entier, ou vide.'];
  }
  if (min !== null && max !== null && min > max) {
    return ['Le minimum de nuits ne peut pas dépasser le maximum.'];
  }
  return [];
}

/** The form → the promotion, or the problems to fix (the API checks again). */
export function promotionFrom(draft: PromotionDraft): {
  promotion: Promotion | null;
  errors: string[];
} {
  const errors: string[] = [];
  if (!draft.nameFr.trim()) {
    errors.push('Le nom en français est obligatoire.');
  }
  const min = bound(draft.min);
  const max = bound(draft.max);
  errors.push(...boundErrors(min, max));
  const percent = draft.kind === 'percent' ? parsePercent(draft.value) : null;
  const amount = draft.kind === 'percent' ? null : parseEuros(draft.value);
  if (draft.kind === 'percent' && (percent === null || percent === 0)) {
    errors.push('Remise : un pourcentage entre 0 et 100.');
  }
  if (draft.kind !== 'percent' && (amount === null || amount === 0)) {
    errors.push('Remise : un montant en euros supérieur à 0.');
  }
  if (draft.from && draft.until && draft.from > draft.until) {
    errors.push('La date de début doit être avant la date de fin.');
  }
  if (errors.length > 0) {
    return { promotion: null, errors };
  }
  return {
    errors,
    promotion: {
      name: { fr: draft.nameFr.trim(), en: draft.nameEn.trim() },
      discount:
        draft.kind === 'percent'
          ? { kind: 'percent', bp: percent! }
          : { kind: draft.kind, amount: amount! },
      min_nights: min ?? null,
      max_nights: max ?? null,
      valid_from: draft.from || null,
      valid_until: draft.until || null,
      active: draft.active,
    },
  };
}

/** The form → the option, or the problems to fix. */
export function optionFrom(draft: OptionDraft): { option: StayOption | null; errors: string[] } {
  const errors: string[] = [];
  if (!draft.nameFr.trim()) {
    errors.push('Le nom en français est obligatoire.');
  }
  const price = parseEuros(draft.price || '0');
  if (price === null) {
    errors.push('Prix : un montant en euros (0 si offert).');
  }
  const min = bound(draft.min);
  const max = bound(draft.max);
  errors.push(...boundErrors(min, max));
  const position = /^\d+$/.test(draft.position.trim()) ? Number(draft.position) : null;
  if (position === null) {
    errors.push("Ordre d'affichage : un nombre entier.");
  }
  if (errors.length > 0) {
    return { option: null, errors };
  }
  return {
    errors,
    option: {
      name: { fr: draft.nameFr.trim(), en: draft.nameEn.trim() },
      description: { fr: draft.descriptionFr.trim(), en: draft.descriptionEn.trim() },
      unit: draft.unit,
      price: price!,
      min_nights: min ?? null,
      max_nights: max ?? null,
      position: position!,
      active: draft.active,
    },
  };
}
