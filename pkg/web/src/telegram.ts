import {
  init as telegramInit,
  initData,
  hapticFeedbackImpactOccurred,
  hapticFeedbackNotificationOccurred,
  hapticFeedbackSelectionChanged,
  viewport,
} from "@telegram-apps/sdk";
import { isTMA } from "@telegram-apps/bridge";

export function setup() {
  // Only initialize when running inside Telegram
  if (!isTMA()) {
    return;
  }

  telegramInit();
  initData.restore();
}

/**
 * Get the raw init data from the Telegram Mini-App environment.
 * Returns null if not running in a mini-app.
 */
export function getRawInitData(): string | null {
  if (!isTMA()) {
    return null;
  }

  return initData.raw() ?? null;
}

export type TgHaptic =
  | { type: "impact"; style: "light" | "medium" | "heavy" | "rigid" | "soft" }
  | { type: "notification"; style: "success" | "warning" | "error" }
  | { type: "selection" };

/**
 * Play haptic feedback on the user's device.
 * If we can't, this is a no-op.
 */
export function playHapticFeedback(h: TgHaptic) {
  if (!isTMA()) {
    return;
  }

  if (h.type === "impact") {
    if (hapticFeedbackImpactOccurred.isAvailable()) {
      hapticFeedbackImpactOccurred(h.style);
    }
    return;
  }

  if (h.type === "notification") {
    if (hapticFeedbackNotificationOccurred.isAvailable()) {
      hapticFeedbackNotificationOccurred(h.style);
    }
    return;
  }

  if (h.type === "selection") {
    if (hapticFeedbackSelectionChanged.isAvailable()) {
      hapticFeedbackSelectionChanged();
    }
    return;
  }

  const _exhaustiveCheck: never = h;
  throw new Error(`Unhandled haptic feedback type: ${_exhaustiveCheck}`);
}

/** Expand the mini-app to full screen, if possible. */
export function maximize() {
  if (viewport.expand.isAvailable()) {
    viewport.expand();
  }
}
