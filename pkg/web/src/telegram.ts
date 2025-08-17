import { init as telegramInit, initData } from "@telegram-apps/sdk";
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
}

/**
 * Get the raw init data.
 *
 * If we're running in the mock environment, returns the mock init data.
 */
export function getRawInitData() {
  if (isMockEnv) {
    const mockInitData = process.env.TG_MOCK_INIT_DATA;
    if (!mockInitData) {
      throw new Error(
        "We're running in a mock environment, but TG_MOCK_INIT_DATA is not set.",
      );
    }
    return process.env.TG_MOCK_INIT_DATA;
  }

  return initData.raw();
}
