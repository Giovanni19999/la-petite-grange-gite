import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

export type SmtpSecurity = 'starttls' | 'tls' | 'none';

/** Settings of the email and SMS sending (F025); the secrets are never sent by the API. */
export interface MessagingSettings {
  smtp: {
    host: string;
    port: number;
    security: SmtpSecurity;
    username: string;
    from_address: string;
    from_name: string;
    reply_to: string | null;
  };
  sms_enabled: boolean;
  sms_sender: string;
  owner_emails: string[];
  owner_phones: string[];
  max_attempts: number;
  first_retry_minutes: number;
}

export interface MessagingState {
  settings: MessagingSettings;
  /** The SMTP password is set in the server configuration. */
  smtp_password_set: boolean;
  /** An SMS provider (Brevo) is configured on the server. */
  sms_provider: boolean;
}

export type MessageStatus = 'pending' | 'sending' | 'sent' | 'failed' | 'to_check' | 'disabled';

/** A message of the sending queue (subject of an email or start of an SMS). */
export interface SentMessage {
  id: number;
  channel: 'email' | 'sms';
  recipient: string;
  summary: string;
  status: MessageStatus;
  attempts: number;
  error: string | null;
  created_at: string;
  sent_at: string | null;
}

export interface TestResult {
  recipient: string;
  sent: boolean;
  error: string | null;
}

const BASE = '/api/admin/messaging';

/** Sending API (F025). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class MessagingApi {
  private readonly http = inject(HttpClient);

  state(): Observable<MessagingState> {
    return this.http.get<MessagingState>(BASE);
  }

  save(settings: MessagingSettings): Observable<void> {
    return this.http.put<void>(BASE, settings);
  }

  messages(): Observable<SentMessage[]> {
    return this.http.get<SentMessage[]>(`${BASE}/messages`);
  }

  problems(): Observable<{ count: number }> {
    return this.http.get<{ count: number }>(`${BASE}/problems`);
  }

  resend(id: number): Observable<void> {
    return this.http.post<void>(`${BASE}/messages/${id}/resend`, {});
  }

  testEmail(): Observable<TestResult[]> {
    return this.http.post<TestResult[]>(`${BASE}/test-email`, {});
  }

  testSms(): Observable<TestResult[]> {
    return this.http.post<TestResult[]>(`${BASE}/test-sms`, {});
  }
}
