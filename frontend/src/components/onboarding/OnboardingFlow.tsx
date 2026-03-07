import React, { useEffect } from 'react';
import { useOnboarding } from '@/contexts/OnboardingContext';
import {
  WelcomeStep,
  ModeSelectionStep,
  PermissionsStep,
  DownloadProgressStep,
  SetupOverviewStep,
} from './steps';

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const { currentStep, processingMode } = useOnboarding();
  const [isMac, setIsMac] = React.useState(false);

  useEffect(() => {
    // Check if running on macOS
    const checkPlatform = async () => {
      try {
        // Dynamic import to avoid SSR issues if any
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        console.error('Failed to detect platform:', e);
        // Fallback
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  // 5-Step Onboarding Flow:
  // Step 1: Welcome
  // Step 2: Mode Selection (Local vs Hosted/Gemini)
  // Step 3: Setup Overview (local only)
  // Step 4: Download Progress (local only)
  // Step 5: Permissions (macOS only, local mode)
  // Hosted mode completes at Step 2 (skips 3-5)

  return (
    <div className="onboarding-flow">
      {currentStep === 1 && <WelcomeStep />}
      {currentStep === 2 && <ModeSelectionStep />}
      {currentStep === 3 && processingMode === 'local' && <SetupOverviewStep />}
      {currentStep === 4 && processingMode === 'local' && <DownloadProgressStep />}
      {currentStep === 5 && isMac && processingMode === 'local' && <PermissionsStep />}
    </div>
  );
}
