const KEY = "medhealth_auth_demo";

export type AuthState = { email: string } | null;

export function getAuth(): AuthState {
  const raw = localStorage.getItem(KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as AuthState;
  } catch {
    return null;
  }
}

export function setAuth(email: string) {
  localStorage.setItem(KEY, JSON.stringify({ email }));
}

export function clearAuth() {
  localStorage.removeItem(KEY);
}
