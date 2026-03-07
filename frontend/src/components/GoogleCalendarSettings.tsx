"use client";

import { useCallback, useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, RefreshCw, Unplug } from "lucide-react";
import { toast } from "sonner";
import {
  calendarService,
  CalendarStatusResponse,
} from "@/services/calendarService";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

function formatDateTime(value: string | null | undefined): string {
  if (!value) return "Never";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

export function GoogleCalendarSettings() {
  const [status, setStatus] = useState<CalendarStatusResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const [isUpgradingAccess, setIsUpgradingAccess] = useState(false);
  const hasAccount = Boolean(status?.account);

  const loadStatus = useCallback(async () => {
    try {
      const nextStatus = await calendarService.getStatus();
      setStatus(nextStatus);
    } catch (error) {
      console.error("Failed to load Google Calendar status:", error);
      toast.error("Failed to load Google Calendar status");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      const nextStatus = await calendarService.connectGoogle(false);
      setStatus(nextStatus);
      toast.success("Google Calendar connected");
    } catch (error) {
      console.error("Failed to connect Google Calendar:", error);
      toast.error("Failed to connect Google Calendar", {
        description: String(error),
      });
    } finally {
      setIsConnecting(false);
    }
  };

  const handleSync = async () => {
    setIsSyncing(true);
    try {
      const result = await calendarService.syncNow();
      await loadStatus();
      toast.success("Google Calendar synced", {
        description: `${result.synced_events} events refreshed`,
      });
    } catch (error) {
      console.error("Failed to sync Google Calendar:", error);
      toast.error("Failed to sync Google Calendar", {
        description: String(error),
      });
    } finally {
      setIsSyncing(false);
    }
  };

  const handleDisconnect = async () => {
    setIsDisconnecting(true);
    try {
      const nextStatus = await calendarService.disconnectGoogle();
      setStatus(nextStatus);
      toast.success("Google Calendar disconnected");
    } catch (error) {
      console.error("Failed to disconnect Google Calendar:", error);
      toast.error("Failed to disconnect Google Calendar", {
        description: String(error),
      });
    } finally {
      setIsDisconnecting(false);
    }
  };

  const handleUpgradeAccess = async () => {
    setIsUpgradingAccess(true);
    try {
      const nextStatus = await calendarService.upgradeGoogleAccess();
      setStatus(nextStatus);
      toast.success("Google Calendar write access enabled");
    } catch (error) {
      console.error("Failed to upgrade Google Calendar access:", error);
      toast.error("Failed to upgrade Google Calendar access", {
        description: String(error),
      });
    } finally {
      setIsUpgradingAccess(false);
    }
  };

  if (isLoading) {
    return <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">Loading Google Calendar...</div>;
  }

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Google Calendar</h3>
          <p className="text-sm text-gray-600 mt-1">
            Sync your primary Google Calendar and automatically link live meeting notes to the right event.
          </p>
        </div>
        {status?.connected ? (
          <div className="inline-flex items-center gap-2 rounded-full bg-green-50 px-3 py-1 text-sm text-green-700">
            <CheckCircle2 className="h-4 w-4" />
            Connected
          </div>
        ) : (
          <div className="inline-flex items-center gap-2 rounded-full bg-gray-100 px-3 py-1 text-sm text-gray-600">
            <Unplug className="h-4 w-4" />
            Not connected
          </div>
        )}
      </div>

      {!status?.client_configured && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>OAuth client ID missing</AlertTitle>
          <AlertDescription>
            Set <code>FRIDAY_GOOGLE_CLIENT_ID</code> in the app environment to enable Google Calendar sign-in.
          </AlertDescription>
        </Alert>
      )}

      {status?.account && (
        <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="grid gap-3 text-sm text-gray-700 sm:grid-cols-2">
            <div>
              <div className="font-medium text-gray-900">Account</div>
              <div>{status.account.email || "Unknown account"}</div>
            </div>
            <div>
              <div className="font-medium text-gray-900">Status</div>
              <div className="capitalize">{status.account.connection_status}</div>
            </div>
            <div>
              <div className="font-medium text-gray-900">Last sync</div>
              <div>{formatDateTime(status.account.last_sync_at)}</div>
            </div>
            <div>
              <div className="font-medium text-gray-900">Scope</div>
              <div>{status.account.scopes.join(", ") || "None"}</div>
            </div>
            <div>
              <div className="font-medium text-gray-900">Write access</div>
              <div>{status.can_write ? "Enabled" : "Read-only"}</div>
            </div>
          </div>
          {status.account.last_error && (
            <div className="mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
              {status.account.last_error}
            </div>
          )}
        </div>
      )}

      <div className="flex flex-wrap gap-3">
        {!status?.connected ? (
          <>
            <Button
              variant="blue"
              onClick={handleConnect}
              disabled={!status?.client_configured || isConnecting}
            >
              {isConnecting
                ? "Waiting for Google sign-in..."
                : hasAccount
                  ? "Reconnect Google Calendar"
                  : "Connect Google Calendar"}
            </Button>
            {hasAccount && (
              <Button
                variant="destructive"
                onClick={handleDisconnect}
                disabled={isDisconnecting}
              >
                {isDisconnecting ? "Disconnecting..." : "Clear Google Calendar connection"}
              </Button>
            )}
          </>
        ) : (
          <>
            <Button variant="outline" onClick={handleSync} disabled={isSyncing || status.syncing}>
              <RefreshCw className={`h-4 w-4 ${isSyncing || status.syncing ? "animate-spin" : ""}`} />
              {isSyncing || status.syncing ? "Syncing..." : "Sync now"}
            </Button>
            {!status.can_write && (
              <Button variant="outline" onClick={handleUpgradeAccess} disabled={isUpgradingAccess}>
                {isUpgradingAccess ? "Waiting for Google..." : "Enable event creation"}
              </Button>
            )}
            <Button
              variant="destructive"
              onClick={handleDisconnect}
              disabled={isDisconnecting}
            >
              {isDisconnecting ? "Disconnecting..." : "Disconnect"}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
