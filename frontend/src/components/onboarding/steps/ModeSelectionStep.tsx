'use client';

import React, { useState } from 'react';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { OnboardingContainer } from '../OnboardingContainer';
import { Button } from '@/components/ui/button';
import { Shield, Zap, Key, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

export function ModeSelectionStep() {
  const { goNext, setProcessingMode, completeOnboardingHosted } = useOnboarding();
  const [selected, setSelected] = useState<'local' | 'hosted' | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [isCompleting, setIsCompleting] = useState(false);

  const handleContinue = async () => {
    if (selected === 'local') {
      setProcessingMode('local');
      goNext();
    } else if (selected === 'hosted') {
      if (!apiKey.trim()) {
        toast.error('Please enter your Gemini API key');
        return;
      }
      setIsCompleting(true);
      try {
        setProcessingMode('hosted');
        await completeOnboardingHosted(apiKey.trim());
        await new Promise(resolve => setTimeout(resolve, 100));
        window.location.reload();
      } catch (error) {
        console.error('Failed to complete hosted onboarding:', error);
        toast.error('Failed to complete setup', {
          description: String(error),
        });
        setIsCompleting(false);
      }
    }
  };

  const modes = [
    {
      id: 'local' as const,
      icon: <Shield className="w-6 h-6" />,
      title: '100% Local',
      description: 'All processing on your machine. Requires downloading ~1.5 GB of AI models.',
      badge: 'Privacy First',
    },
    {
      id: 'hosted' as const,
      icon: <Zap className="w-6 h-6" />,
      title: 'Google Gemini (Hosted)',
      description: 'Uses Gemini for summaries. Local transcription, no large model downloads needed.',
      badge: 'Best Quality',
    },
  ];

  return (
    <OnboardingContainer
      title="Choose Your Processing Mode"
      description="You can always change this later in Settings."
      step={2}
      hideProgress={true}
    >
      <div className="flex flex-col items-center space-y-6">
        <div className="w-full max-w-md space-y-3">
          {modes.map((mode) => (
            <button
              key={mode.id}
              onClick={() => setSelected(mode.id)}
              className={`w-full text-left p-4 rounded-lg border-2 transition-all ${
                selected === mode.id
                  ? 'border-gray-900 bg-gray-50'
                  : 'border-gray-200 hover:border-gray-300 bg-white'
              }`}
            >
              <div className="flex items-start gap-3">
                <div className={`mt-0.5 ${selected === mode.id ? 'text-gray-900' : 'text-gray-400'}`}>
                  {mode.icon}
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="font-medium text-gray-900">{mode.title}</h3>
                    <span className="text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-600">
                      {mode.badge}
                    </span>
                  </div>
                  <p className="text-sm text-gray-500 mt-1">{mode.description}</p>
                </div>
              </div>
            </button>
          ))}
        </div>

        {/* API Key input for hosted mode */}
        {selected === 'hosted' && (
          <div className="w-full max-w-md space-y-2">
            <label className="text-sm font-medium text-gray-700 flex items-center gap-1.5">
              <Key className="w-3.5 h-3.5" />
              Gemini API Key
            </label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Enter your GEMINI_API_KEY"
              className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent"
            />
            <p className="text-xs text-gray-500">
              Get your key from{' '}
              <a
                href="https://aistudio.google.com/apikey"
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:underline"
              >
                Google AI Studio
              </a>
            </p>
          </div>
        )}

        <div className="w-full max-w-xs">
          <Button
            onClick={handleContinue}
            disabled={!selected || isCompleting || (selected === 'hosted' && !apiKey.trim())}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white disabled:opacity-50"
          >
            {isCompleting ? (
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            ) : selected === 'hosted' ? (
              'Complete Setup'
            ) : (
              'Continue'
            )}
          </Button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
