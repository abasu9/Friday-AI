const ONBOARDING_DEMO_MODE_KEY = 'friday-demo-onboarding';

export function isOnboardingDemoModeEnabled(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  return window.localStorage.getItem(ONBOARDING_DEMO_MODE_KEY) === '1';
}

export function setOnboardingDemoModeEnabled(enabled: boolean): void {
  if (typeof window === 'undefined') {
    return;
  }

  if (enabled) {
    window.localStorage.setItem(ONBOARDING_DEMO_MODE_KEY, '1');
    return;
  }

  window.localStorage.removeItem(ONBOARDING_DEMO_MODE_KEY);
}

export function enableOnboardingDemoMode(): void {
  setOnboardingDemoModeEnabled(true);
}

export function disableOnboardingDemoMode(): void {
  setOnboardingDemoModeEnabled(false);
}
