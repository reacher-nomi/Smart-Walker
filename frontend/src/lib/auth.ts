/**
 * Authentication utilities for managing user session
 */

const AUTH_KEY = "medhealth_auth";

export interface AuthData {
  email: string;
  timestamp: number;
}

/**
 * Set authentication data (stores email in localStorage)
 */
export function setAuth(email: string): void {
  const authData: AuthData = {
    email,
    timestamp: Date.now(),
  };
  localStorage.setItem(AUTH_KEY, JSON.stringify(authData));
}

/**
 * Get authentication data from localStorage
 */
export function getAuth(): AuthData | null {
  try {
    const stored = localStorage.getItem(AUTH_KEY);
    if (!stored) return null;
    
    const authData: AuthData = JSON.parse(stored);
    return authData;
  } catch {
    return null;
  }
}

/**
 * Clear authentication data
 */
export function clearAuth(): void {
  localStorage.removeItem(AUTH_KEY);
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
  return getAuth() !== null;
}

/**
 * Get authenticated user email
 */
export function getAuthEmail(): string | null {
  const auth = getAuth();
  return auth?.email || null;
}
