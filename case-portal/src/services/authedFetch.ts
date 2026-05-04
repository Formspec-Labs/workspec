/**
 * Authenticated fetch helper for talking to a JWT-backed wos-server.
 *
 * Contract:
 * - `authedFetch` is a drop-in replacement for `fetch` that attaches
 *   `Authorization: Bearer <token>` when a token has been stored.
 * - Login responses from `wos-server` are a `TokenPair`:
 *     `{ accessToken, refreshToken, accessExpiresAt, refreshExpiresAt, user }`
 *   `storeLogin` below caches them in `sessionStorage`; `storeLogout`
 *   clears them.
 * - When the studio boots against a mock-auth server (the default), this
 *   helper is a no-op — `fetch` is called directly and the `Authorization`
 *   header is omitted.
 * - On a 401 response `authedFetch` clears the cached token so the
 *   studio's `AuthGate` re-prompts on the next call.
 */

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  accessExpiresAt: string;
  refreshExpiresAt: string;
  user: {
    id: string;
    name: string;
    email: string;
    role: string;
    avatar?: string;
  };
}

const ACCESS_KEY = 'wos.auth.accessToken';
const REFRESH_KEY = 'wos.auth.refreshToken';

export function getAccessToken(): string | null {
  try {
    return sessionStorage.getItem(ACCESS_KEY);
  } catch {
    return null;
  }
}

export function getRefreshToken(): string | null {
  try {
    return sessionStorage.getItem(REFRESH_KEY);
  } catch {
    return null;
  }
}

export function storeLogin(pair: TokenPair): void {
  try {
    sessionStorage.setItem(ACCESS_KEY, pair.accessToken);
    sessionStorage.setItem(REFRESH_KEY, pair.refreshToken);
  } catch {
    // storage may be disabled in some browsers; fall back to in-memory only
  }
}

export function storeLogout(): void {
  try {
    sessionStorage.removeItem(ACCESS_KEY);
    sessionStorage.removeItem(REFRESH_KEY);
  } catch {
    // ignore
  }
}

export async function authedFetch(
  input: RequestInfo | URL,
  init: RequestInit = {},
): Promise<Response> {
  const token = getAccessToken();
  const headers = new Headers(init.headers || {});
  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  const res = await fetch(input, { ...init, headers });
  if (res.status === 401) {
    storeLogout();
  }
  return res;
}
