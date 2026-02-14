import * as tg from "./telegram";
import { redirect } from "@tanstack/react-router";

export interface AuthCreds {
  token: string;
}

const CREDS_STORAGE_KEY = "zap_auth_creds";

/**
 * Check for intrinsic credentials from the Telegram Mini-App environment.
 * Returns null if not running in TMA or no init data available.
 */
function getIntrinsicAuth(): AuthCreds | null {
  try {
    const raw = tg.getRawInitData();
    if (raw) {
      return { token: `tg_init_data:${raw}` };
    }
  } catch {
    // Not in TMA or SDK not available
  }
  return null;
}

/** Store credentials in localStorage. */
export function setCredentials(creds: AuthCreds): void {
  localStorage.setItem(CREDS_STORAGE_KEY, JSON.stringify(creds));
}

/** Remove stored credentials from localStorage. */
export function removeCredentials(): void {
  localStorage.removeItem(CREDS_STORAGE_KEY);
}

/**
 * Get the authentication credentials, if available.
 * Checks localStorage first, then intrinsic (TMA) auth, then returns null.
 */
export async function getAuth(): Promise<AuthCreds | null> {
  // Check localStorage for stored credentials
  try {
    const stored = localStorage.getItem(CREDS_STORAGE_KEY);
    if (stored) {
      const creds: AuthCreds = JSON.parse(stored);
      if (creds.token) {
        return creds;
      }
    }
  } catch {
    // Bad data in storage, clean it up
    localStorage.removeItem(CREDS_STORAGE_KEY);
  }

  // Check for intrinsic TMA credentials
  const intrinsic = getIntrinsicAuth();
  if (intrinsic) {
    return intrinsic;
  }

  return null;
}

/**
 * TanStack Router `beforeLoad` hook to check a route is authenticated.
 * Redirects to `/login?redirect=...` if not authenticated.
 */
export async function checkAuthBeforeLoad() {
  const auth = await getAuth();
  if (!auth) {
    throw redirect({
      to: "/login",
      search: { redirect: location.pathname + location.search + location.hash },
    });
  }
}
