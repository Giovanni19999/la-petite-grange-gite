import { IsoDate, parseIso } from '../../../shared/dates/dates';
import { Lang } from '../pages/data-access/pages.models';

/** Texts of the booking and tracking pages (F007), in French and English. */
export const BOOKING_TEXT = {
  fr: {
    title: 'Réserver',
    back: '← Revenir au calendrier',
    stay: 'Votre séjour',
    from: (arrival: string, departure: string) => `Du ${arrival} au ${departure}`,
    guests: (adults: number, children: number) =>
      `${adults} adulte${adults > 1 ? 's' : ''}` +
      (children > 0 ? `, ${children} enfant${children > 1 ? 's' : ''}` : ''),
    nights: (n: number) => (n > 1 ? `${n} nuits` : '1 nuit'),
    promotion: (name: string) => `Promotion « ${name} »`,
    cleaning: 'Ménage',
    total: 'Prix du séjour',
    deposit: 'Acompte, à régler dès la réservation',
    balance: (date: string) => `Solde, à régler avant le ${date}`,
    allAtBooking: 'La totalité est à régler dès la réservation.',
    tax: 'Taxe de séjour, à régler sur place',
    rules: 'Règles du gîte',
    arrivalFrom: (time: string) => `Arrivée à partir de ${time}`,
    departureBefore: (time: string) => `Départ avant ${time}`,
    capacity: (n: number) => `${n} personnes au maximum`,
    pets: (allowed: boolean) => (allowed ? 'Animaux acceptés' : 'Animaux non acceptés'),
    howToPay: 'Comment payer',
    noOnlinePayment:
      'Pas de paiement en ligne : votre réservation est confirmée tout de suite, vous réglez l’acompte puis le solde comme indiqué ci-dessous.',
    document: 'Le RIB est joint à l’e-mail de confirmation.',
    details: 'Vos coordonnées',
    firstName: 'Prénom',
    lastName: 'Nom',
    email: 'E-mail',
    phone: 'Téléphone mobile (format international)',
    street: 'Adresse',
    postcode: 'Code postal',
    town: 'Ville',
    country: 'Pays',
    defaultCountry: 'France',
    arrivalTime: 'Heure d’arrivée prévue',
    message: 'Message aux propriétaires (facultatif)',
    accept: 'J’ai lu et j’accepte les',
    terms: 'conditions de location',
    confirm: 'Confirmer la réservation',
    sending: 'Réservation en cours…',
    closed: 'La réservation en ligne n’est pas encore ouverte. Contactez-nous pour réserver.',
    refused: 'La réservation n’a pas pu être faite :',
    error: 'La réservation n’a pas pu être faite, réessayez plus tard.',
    missingStay: 'Choisissez d’abord vos dates dans le calendrier.',
    tracking: 'Votre réservation',
    confirmed: (reference: string) =>
      `Merci ! Votre réservation ${reference} est confirmée. Un e-mail de confirmation vous a été envoyé.`,
    reference: 'Référence',
    status: {
      confirmee: 'Confirmée, acompte attendu',
      acompte_recu: 'Confirmée, acompte reçu',
      soldee: 'Confirmée, tout est payé',
      annulee: 'Annulée',
    },
    paid: 'Déjà reçu',
    changeTime: 'Changer l’heure d’arrivée',
    timeSaved: 'Heure d’arrivée enregistrée, les propriétaires sont prévenus.',
    tooLate: 'L’heure d’arrivée ne peut plus être changée en ligne : contactez les propriétaires.',
    notFound: 'Réservation introuvable. Vérifiez le lien reçu par e-mail.',
    save: 'Enregistrer',
  },
  en: {
    title: 'Book',
    back: '← Back to the calendar',
    stay: 'Your stay',
    from: (arrival: string, departure: string) => `From ${arrival} to ${departure}`,
    guests: (adults: number, children: number) =>
      `${adults} adult${adults > 1 ? 's' : ''}` +
      (children > 0 ? `, ${children} child${children > 1 ? 'ren' : ''}` : ''),
    nights: (n: number) => (n > 1 ? `${n} nights` : '1 night'),
    promotion: (name: string) => `Offer “${name}”`,
    cleaning: 'Cleaning',
    total: 'Price of the stay',
    deposit: 'Deposit, to pay at booking',
    balance: (date: string) => `Balance, to pay by ${date}`,
    allAtBooking: 'The full amount is to pay at booking.',
    tax: 'Tourist tax, paid on arrival',
    rules: 'House rules',
    arrivalFrom: (time: string) => `Arrival from ${time}`,
    departureBefore: (time: string) => `Departure before ${time}`,
    capacity: (n: number) => `${n} guests at most`,
    pets: (allowed: boolean) => (allowed ? 'Pets welcome' : 'No pets'),
    howToPay: 'How to pay',
    noOnlinePayment:
      'No online payment: your booking is confirmed at once, you pay the deposit then the balance as explained below.',
    document: 'The bank details are attached to the confirmation email.',
    details: 'Your details',
    firstName: 'First name',
    lastName: 'Last name',
    email: 'Email',
    phone: 'Mobile phone (international format)',
    street: 'Address',
    postcode: 'Postcode',
    town: 'Town',
    country: 'Country',
    defaultCountry: '',
    arrivalTime: 'Expected arrival time',
    message: 'Message to the owners (optional)',
    accept: 'I have read and accept the',
    terms: 'rental terms',
    confirm: 'Confirm the booking',
    sending: 'Booking…',
    closed: 'Online booking is not open yet. Please contact us to book.',
    refused: 'The booking could not be made:',
    error: 'The booking could not be made, please try again later.',
    missingStay: 'First choose your dates in the calendar.',
    tracking: 'Your booking',
    confirmed: (reference: string) =>
      `Thank you! Your booking ${reference} is confirmed. A confirmation email has been sent to you.`,
    reference: 'Reference',
    status: {
      confirmee: 'Confirmed, deposit expected',
      acompte_recu: 'Confirmed, deposit received',
      soldee: 'Confirmed, fully paid',
      annulee: 'Cancelled',
    },
    paid: 'Already received',
    changeTime: 'Change the arrival time',
    timeSaved: 'Arrival time saved, the owners are told.',
    tooLate: 'The arrival time can no longer be changed online: please contact the owners.',
    notFound: 'Booking not found. Check the link received by email.',
    save: 'Save',
  },
};

export type BookingText = (typeof BOOKING_TEXT)['fr'];

/** Cents → "260 €" / "€260", with cents when needed. */
export function money(cents: number, lang: Lang): string {
  return new Intl.NumberFormat(lang === 'en' ? 'en-GB' : 'fr-FR', {
    style: 'currency',
    currency: 'EUR',
    minimumFractionDigits: cents % 100 === 0 ? 0 : 2,
  }).format(cents / 100);
}

/** "samedi 10 juillet 2027" / "Saturday 10 July 2027". */
export function longDate(value: IsoDate, lang: Lang): string {
  return new Intl.DateTimeFormat(lang === 'en' ? 'en-GB' : 'fr-FR', {
    timeZone: 'UTC',
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(parseIso(value)!);
}

/** Minutes after midnight → "16:00". */
export function clock(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`;
}
