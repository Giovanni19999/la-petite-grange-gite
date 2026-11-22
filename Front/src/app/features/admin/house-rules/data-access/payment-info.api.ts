import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

/** How guests pay offline (F007): shown on the booking page and in the confirmation email. */
export interface PaymentInfo {
  /** Sanitised HTML in French and English. */
  instructions: { fr: string; en: string };
  /** Methods the owners choose from when they record a payment. */
  methods: string[];
  /** Bank details document attached to the confirmation email. */
  document: { name: string; content_type: string } | null;
  /** Online payment provider configured on the server (`none`: payments are offline). */
  provider: string;
}

const BASE = '/api/admin/payment-info';

/** Payment instructions API (F007). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class PaymentInfoApi {
  private readonly http = inject(HttpClient);

  info(): Observable<PaymentInfo> {
    return this.http.get<PaymentInfo>(BASE);
  }

  save(instructions: { fr: string; en: string }, methods: string[]): Observable<void> {
    return this.http.put<void>(BASE, { instructions, methods });
  }

  uploadDocument(file: File): Observable<void> {
    const form = new FormData();
    form.append('file', file, file.name);
    return this.http.put<void>(`${BASE}/document`, form);
  }

  removeDocument(): Observable<void> {
    return this.http.delete<void>(`${BASE}/document`);
  }

  /** Address of the document, for the owners to check it. */
  readonly documentUrl = `${BASE}/document`;
}
