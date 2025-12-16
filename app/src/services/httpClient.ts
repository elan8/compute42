// Minimal HTTP client wrapper using fetch with JSON helpers and token injection

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface HttpClientOptions {
  baseUrl?: string;
  getToken?: () => string | null | undefined | Promise<string | null | undefined>;
}

export class HttpClient {
  private readonly baseUrl: string;
  private readonly getToken?: () => string | null | undefined | Promise<string | null | undefined>;

  constructor(options: HttpClientOptions = {}) {
    this.baseUrl = options.baseUrl ?? '';
    this.getToken = options.getToken;
  }

  async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const url = this.baseUrl ? `${this.baseUrl}${path}` : path;
    const headers = new Headers(init.headers);
    if (!headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
    if (!headers.has('Accept')) headers.set('Accept', 'application/json');
    const token = await this.getToken?.();
    if (token) headers.set('Authorization', `Bearer ${token}`);
    const resp = await fetch(url, { ...init, headers });
    if (!resp.ok) {
      const text = await resp.text().catch(() => '');
      throw new Error(text || `HTTP ${resp.status}`);
    }
    const contentType = resp.headers.get('content-type') || '';
    if (contentType.includes('application/json')) return (await resp.json()) as T;
    return (await resp.text()) as unknown as T;
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'GET' });
  }

  post<T, B = unknown>(path: string, body?: B): Promise<T> {
    return this.request<T>(path, { method: 'POST', body: body ? JSON.stringify(body) : undefined });
  }

  put<T, B = unknown>(path: string, body?: B): Promise<T> {
    return this.request<T>(path, { method: 'PUT', body: body ? JSON.stringify(body) : undefined });
  }

  patch<T, B = unknown>(path: string, body?: B): Promise<T> {
    return this.request<T>(path, {
      method: 'PATCH',
      body: body ? JSON.stringify(body) : undefined,
    });
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'DELETE' });
  }
}

import { authService } from './authService';

export const httpClient = new HttpClient({
  baseUrl: '',
  getToken: async () => await authService.getAccessToken(),
});
