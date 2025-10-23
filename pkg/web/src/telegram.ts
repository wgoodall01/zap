import {
  init as telegramInit,
  initData,
  hapticFeedbackImpactOccurred,
  hapticFeedbackNotificationOccurred,
  hapticFeedbackSelectionChanged,
  viewport,
} from "@telegram-apps/sdk";
import { isTMA } from "@telegram-apps/bridge";

// Check whether we're running in a mock environment (i.e. a non-Telegram-hosted browser context)
const isMockEnv = process.env.NODE_ENV === "development" && !isTMA();

export function setup() {
  // In mock env, don't initialize Telegram SDK
  if (isMockEnv) {
    return;
  }

  // Initialize Telegram SDK
  telegramInit();

  // Mount components
  initData.restore();
}

/**
 * Get the raw init data.
 *
 * If we're running in the mock environment, returns the mock init data.
 */
export function getRawInitData(): string | null {
  if (isMockEnv) {
    const mockInitData = process.env.TG_MOCK_INIT_DATA;
    if (!mockInitData) {
      throw new Error("We're running in a mock environment, but TG_MOCK_INIT_DATA is not set.");
    }
    return process.env.TG_MOCK_INIT_DATA ?? null;
  }

  // If we're running outside of a mini-app, we don't have init data.
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
  if (isMockEnv) {
    console.log("tg mock: haptic feedback:", h);
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
