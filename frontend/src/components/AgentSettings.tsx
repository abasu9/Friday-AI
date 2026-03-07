"use client";

import { useCallback, useEffect, useState } from "react";
import { BrainCircuit, CalendarPlus, Loader2, Sparkles } from "lucide-react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { agentService, AgentSettingsPayload, AgentStatusResponse } from "@/services/agentService";
import { calendarService } from "@/services/calendarService";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";

const DEFAULT_SETTINGS: AgentSettingsPayload = {
  enabled: false,
  provider: "gemini",
  model: "gemini-2.5-flash",
  notifications_enabled: true,
  calendar_proposals_enabled: false,
  heartbeat_interval_minutes: 5,
};

function formatDateTime(value: string | null | undefined): string {
  if (!value) return "Never";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

export function AgentSettings() {
  const router = useRouter();
  const [status, setStatus] = useState<AgentStatusResponse | null>(null);
  const [settings, setSettings] = useState<AgentSettingsPayload>(DEFAULT_SETTINGS);
  const [geminiApiKey, setGeminiApiKey] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isSavingApiKey, setIsSavingApiKey] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [isUpgradingCalendar, setIsUpgradingCalendar] = useState(false);

  const loadStatus = useCallback(async () => {
    try {
      const nextStatus = await agentService.getStatus();
      setStatus(nextStatus);
      setSettings(nextStatus.settings);
    } catch (error) {
      console.error("Failed to load agent status:", error);
      toast.error("Failed to load agent status", { description: String(error) });
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const saveSettings = async (nextSettings: AgentSettingsPayload) => {
    setIsSaving(true);
    try {
      const nextStatus = await agentService.setSettings(nextSettings);
      setStatus(nextStatus);
      setSettings(nextStatus.settings);
      toast.success("Agent settings saved");
    } catch (error) {
      console.error("Failed to save agent settings:", error);
      toast.error("Failed to save agent settings", { description: String(error) });
    } finally {
      setIsSaving(false);
    }
  };

  const handleToggle = (key: keyof AgentSettingsPayload, value: boolean | number | string) => {
    const nextSettings = {
      ...settings,
      [key]: value,
    };
    setSettings(nextSettings);
    void saveSettings(nextSettings);
  };

  const handleSaveApiKey = async () => {
    if (!geminiApiKey.trim()) {
      toast.error("Enter a Gemini API key first");
      return;
    }
    setIsSavingApiKey(true);
    try {
      await agentService.saveGeminiApiKey(geminiApiKey.trim());
      setGeminiApiKey("");
      await loadStatus();
      toast.success("Gemini API key saved");
    } catch (error) {
      console.error("Failed to save Gemini API key:", error);
      toast.error("Failed to save Gemini API key", { description: String(error) });
    } finally {
      setIsSavingApiKey(false);
    }
  };

  const handleClearApiKey = async () => {
    setIsSavingApiKey(true);
    try {
      await agentService.clearGeminiApiKey();
      await loadStatus();
      toast.success("Gemini API key cleared");
    } catch (error) {
      console.error("Failed to clear Gemini API key:", error);
      toast.error("Failed to clear Gemini API key", { description: String(error) });
    } finally {
      setIsSavingApiKey(false);
    }
  };

  const handleRunNow = async () => {
    setIsRunning(true);
    try {
      const previousPending = status?.pending_recommendations ?? 0;
      const previousTasks = status?.open_tasks ?? 0;
      const nextStatus = await agentService.runHeartbeatNow();
      setStatus(nextStatus);
      const newPending = Math.max(0, nextStatus.pending_recommendations - previousPending);
      const newTasks = Math.max(0, nextStatus.open_tasks - previousTasks);
      if (newPending > 0 || newTasks > 0) {
        toast.success(
          newPending > 0
            ? `Added ${newPending} new calendar proposal${newPending === 1 ? "" : "s"}`
            : `Added ${newTasks} new task${newTasks === 1 ? "" : "s"}`
        );
      } else {
        toast.success("Agent heartbeat finished", {
          description: "No new Google Calendar proposals were created from the latest meetings.",
        });
      }
    } catch (error) {
      console.error("Failed to run agent heartbeat:", error);
      toast.error("Failed to run agent heartbeat", { description: String(error) });
    } finally {
      setIsRunning(false);
    }
  };

  const handleUpgradeCalendar = async () => {
    setIsUpgradingCalendar(true);
    try {
      await calendarService.upgradeGoogleAccess();
      await loadStatus();
      toast.success("Google Calendar write access enabled");
    } catch (error) {
      console.error("Failed to upgrade Google Calendar access:", error);
      toast.error("Failed to upgrade Google Calendar access", { description: String(error) });
    } finally {
      setIsUpgradingCalendar(false);
    }
  };

  if (isLoading) {
    return <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">Loading agent settings...</div>;
  }

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
            <BrainCircuit className="h-5 w-5 text-blue-600" />
            Agent
          </h3>
          <p className="text-sm text-gray-600 mt-1">
            Persistent meeting memory, proactive recommendations, and approval-based Google Calendar proposals.
          </p>
        </div>
        <Button variant="outline" onClick={() => router.push("/agent")}>
          <Sparkles className="h-4 w-4" />
          Open inbox
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="font-medium text-gray-900">Enable agent</div>
              <div className="text-sm text-gray-600">Allow the heartbeat loop to analyze meetings while the app is open.</div>
            </div>
            <Switch checked={settings.enabled} onCheckedChange={(checked) => handleToggle("enabled", checked)} />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="font-medium text-gray-900">System notifications</div>
              <div className="text-sm text-gray-600">Surface new recommendations with desktop notifications.</div>
            </div>
            <Switch
              checked={settings.notifications_enabled}
              onCheckedChange={(checked) => handleToggle("notifications_enabled", checked)}
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="font-medium text-gray-900">Calendar proposals</div>
              <div className="text-sm text-gray-600">Allow the agent to draft Google Calendar events for approval.</div>
            </div>
            <Switch
              checked={settings.calendar_proposals_enabled}
              onCheckedChange={(checked) => handleToggle("calendar_proposals_enabled", checked)}
            />
          </div>
        </div>

        <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 space-y-3">
          <div className="grid gap-2 text-sm text-gray-700">
            <div className="flex items-center justify-between">
              <span className="font-medium text-gray-900">Model</span>
              <span>{settings.provider} / {settings.model}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-medium text-gray-900">Heartbeat</span>
              <span>Every {settings.heartbeat_interval_minutes} minutes</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-medium text-gray-900">Pending recommendations</span>
              <span>{status?.pending_recommendations ?? 0}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-medium text-gray-900">Open tasks</span>
              <span>{status?.open_tasks ?? 0}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-medium text-gray-900">Last run</span>
              <span>{formatDateTime(status?.last_run_at)}</span>
            </div>
          </div>

          <Button variant="outline" disabled={isRunning || status?.is_running} onClick={handleRunNow}>
            {isRunning || status?.is_running ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
            {isRunning || status?.is_running ? "Running..." : "Run now"}
          </Button>
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 space-y-3">
        <div className="font-medium text-gray-900">Gemini API key</div>
        <div className="text-sm text-gray-600">
          {status?.api_key_configured ? "A Gemini API key is already configured." : "Save a Gemini API key to activate the hosted agent."}
        </div>
        <div className="flex flex-col gap-3 sm:flex-row">
          <Input
            type="password"
            value={geminiApiKey}
            onChange={(event) => setGeminiApiKey(event.target.value)}
            placeholder="AIza..."
          />
          <Button variant="blue" onClick={handleSaveApiKey} disabled={isSavingApiKey}>
            {isSavingApiKey ? "Saving..." : "Save key"}
          </Button>
          {status?.api_key_configured && (
            <Button variant="outline" onClick={handleClearApiKey} disabled={isSavingApiKey}>
              Clear key
            </Button>
          )}
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 space-y-3">
        <div className="flex items-center gap-2 font-medium text-gray-900">
          <CalendarPlus className="h-4 w-4 text-blue-600" />
          Google Calendar write access
        </div>
        <div className="text-sm text-gray-600">
          {status?.calendar_connected
            ? status.calendar_can_write
              ? "Google Calendar is connected with write access."
              : "Google Calendar is connected read-only. Upgrade access to approve event drafts."
            : "Connect Google Calendar first to link meetings and create approved event drafts."}
        </div>
        {status?.calendar_connected && !status?.calendar_can_write && (
          <Button variant="outline" onClick={handleUpgradeCalendar} disabled={isUpgradingCalendar}>
            {isUpgradingCalendar ? "Waiting for Google..." : "Enable calendar write access"}
          </Button>
        )}
      </div>

      {status?.last_error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
          {status.last_error}
        </div>
      )}

      {isSaving && (
        <div className="text-sm text-gray-500">Saving agent settings...</div>
      )}
    </div>
  );
}
