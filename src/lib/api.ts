/**
 * API client utilities for backend communication
 */

const API_BASE_URL = import.meta.env.VITE_API_URL || "http://localhost:8080";

export interface ApiError {
  message: string;
  code?: string;
}

/**
 * Get auth token from localStorage
 */
function getAuthToken(): string | null {
  try {
    const authData = localStorage.getItem("medhealth_auth");
    if (!authData) return null;
    const parsed = JSON.parse(authData);
    return parsed?.token || null;
  } catch {
    return null;
  }
}

/**
 * Generic JSON GET request
 */
export async function apiGetJson<T>(endpoint: string): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const token = getAuthToken();
  
  const response = await fetch(url, {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { "Authorization": `Bearer ${token}` } : {}),
    },
    credentials: "include",
  });

  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      message: `HTTP ${response.status}: ${response.statusText}`,
    }));
    throw new Error(error.message || "Request failed");
  }

  return response.json();
}

/**
 * Generic JSON POST request
 */
export async function apiPostJson<T>(
  endpoint: string,
  data: unknown
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const token = getAuthToken();
  
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { "Authorization": `Bearer ${token}` } : {}),
    },
    credentials: "include",
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      message: `HTTP ${response.status}: ${response.statusText}`,
    }));
    throw new Error(error.message || "Request failed");
  }

  return response.json();
}

/**
 * Generic JSON PUT request
 */
export async function apiPutJson<T>(
  endpoint: string,
  data: unknown
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const response = await fetch(url, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include",
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      message: `HTTP ${response.status}: ${response.statusText}`,
    }));
    throw new Error(error.message || "Request failed");
  }

  return response.json();
}

/**
 * Generic JSON DELETE request
 */
export async function apiDeleteJson<T>(endpoint: string): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;
  const response = await fetch(url, {
    method: "DELETE",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include",
  });

  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      message: `HTTP ${response.status}: ${response.statusText}`,
    }));
    throw new Error(error.message || "Request failed");
  }

  return response.json();
}
