import * as tg from "./telegram";
import { redirect } from "@tanstack/react-router";

export interface AuthCreds {
  token: string;
}

/**
 * Get the authentication profile, if the user is logged in. Otherwise, return null.
 */
export async function getAuth(): Promise<AuthCreds | null> {
  // Get creds from Telegram.
  const tgRawInitData = tg.getRawInitData();
  if (tgRawInitData) {
    return { token: `tg_init_data:${tgRawInitData}` };
  }

  // Otherwise, we're not authenticated.
  return null;
}

/**
 * TanStack Router `beforeLoad` hook to check a root is authenticated.
 * Redirects to `/login?returnTo=...` if not authenticated.
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
