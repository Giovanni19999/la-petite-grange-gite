import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

export type TemplateChannel = 'email' | 'sms';
export type TemplateLang = 'fr' | 'en';

export interface TemplateText {
  lang: TemplateLang;
  /** Empty for an SMS. */
  subject: string;
  /** Sanitised HTML for an email, plain text for an SMS; empty: not sent. */
  body: string;
  /** Differs from the default text. */
  changed: boolean;
}

/** An automatic message the owners can edit (F007, ADR-0021). */
export interface MessageTemplate {
  kind: string;
  label: string;
  recipient: 'guest' | 'owners';
  variables: { name: string; label: string }[];
  channels: { channel: TemplateChannel; texts: TemplateText[] }[];
}

export interface Preview {
  subject: string;
  /** Full email with its layout; `null` for an SMS. */
  html: string | null;
  text: string;
}

export interface TemplateKey {
  kind: string;
  channel: TemplateChannel;
  lang: TemplateLang;
}

const BASE = '/api/admin/templates';

/** Message templates API (F007). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class TemplatesApi {
  private readonly http = inject(HttpClient);

  list(): Observable<MessageTemplate[]> {
    return this.http.get<MessageTemplate[]>(BASE);
  }

  save(key: TemplateKey, subject: string, body: string): Observable<void> {
    return this.http.put<void>(`${BASE}/${key.kind}/${key.channel}/${key.lang}`, {
      subject,
      body,
    });
  }

  restore(key: TemplateKey): Observable<void> {
    return this.http.post<void>(`${BASE}/${key.kind}/${key.channel}/${key.lang}/restore`, {});
  }

  preview(key: TemplateKey, subject: string, body: string): Observable<Preview> {
    return this.http.post<Preview>(`${BASE}/preview`, { ...key, subject, body });
  }
}
