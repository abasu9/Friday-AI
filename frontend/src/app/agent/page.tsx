"use client";

import { useCallback, useEffect, useState } from "react";
import {
  AlertCircle,
  ArrowLeft,
  BrainCircuit,
  CalendarPlus,
  CheckCircle2,
  RefreshCw,
  XCircle,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import {
  agentService,
  AgentRecommendation,
  AgentStatusResponse,
  AgentTask,
} from "@/services/agentService";
import { Button } from "@/components/ui/button";

function formatDateTime(value: string | null | undefined): string {
  if (!value) return "No timestamp";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

export default function AgentInboxPage() {
  const router = useRouter();
  const [status, setStatus] = useState<AgentStatusResponse | null>(null);
  const [recommendations, setRecommendations] = useState<AgentRecommendation[]>([]);
  const [tasks, setTasks] = useState<AgentTask[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [busyRecommendationId, setBusyRecommendationId] = useState<string | null>(null);
  const [busyTaskId, setBusyTaskId] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);

  const load = useCallback(async () => {
    try {
      const [nextStatus, nextRecommendations, nextTasks] = await Promise.all([
        agentService.getStatus(),
        agentService.listRecommendations("pending"),
        agentService.listTasks("open"),
      ]);
      setStatus(nextStatus);
      setRecommendations(nextRecommendations);
      setTasks(nextTasks);
      return {
        status: nextStatus,
        recommendations: nextRecommendations,
        tasks: nextTasks,
      };
    } catch (error) {
      console.error("Failed to load agent inbox:", error);
      toast.error("Failed to load agent inbox", { description: String(error) });
      return null;
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const runNow = async () => {
    setIsRunning(true);
    try {
      const previousPending = status?.pending_recommendations ?? recommendations.length;
      const previousTasks = status?.open_tasks ?? tasks.length;
      const nextStatus = await agentService.runHeartbeatNow();
      setStatus(nextStatus);
      const refreshed = await load();
      const pendingCount = refreshed?.status.pending_recommendations ?? nextStatus.pending_recommendations;
      const openTaskCount = refreshed?.status.open_tasks ?? nextStatus.open_tasks;
      const newPending = Math.max(0, pendingCount - previousPending);
      const newTasks = Math.max(0, openTaskCount - previousTasks);

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

  const acceptRecommendation = async (recommendation: AgentRecommendation) => {
    setBusyRecommendationId(recommendation.id);
    try {
      const result = await agentService.acceptRecommendation(recommendation.id);
      await load();
      toast.success(
        result.created_calendar_event
          ? "Calendar event created"
          : "Recommendation accepted"
      );
    } catch (error) {
      console.error("Failed to accept recommendation:", error);
      toast.error("Failed to accept recommendation", { description: String(error) });
    } finally {
      setBusyRecommendationId(null);
    }
  };

  const dismissRecommendation = async (recommendationId: string) => {
    setBusyRecommendationId(recommendationId);
    try {
      await agentService.dismissRecommendation(recommendationId);
      await load();
      toast.success("Recommendation dismissed");
    } catch (error) {
      console.error("Failed to dismiss recommendation:", error);
      toast.error("Failed to dismiss recommendation", { description: String(error) });
    } finally {
      setBusyRecommendationId(null);
    }
  };

  const completeTask = async (taskId: string) => {
    setBusyTaskId(taskId);
    try {
      await agentService.updateTaskStatus(taskId, "completed");
      await load();
      toast.success("Task marked complete");
    } catch (error) {
      console.error("Failed to update task:", error);
      toast.error("Failed to update task", { description: String(error) });
    } finally {
      setBusyTaskId(null);
    }
  };

  return (
    <div className="h-screen overflow-y-auto bg-background text-foreground">
      <div className="sticky top-0 z-10 bg-background border-b border-border">
        <div className="max-w-6xl mx-auto px-8 py-6 flex items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <button
              onClick={() => router.back()}
              className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
              <span>Back</span>
            </button>
            <div>
              <h1 className="text-3xl font-bold flex items-center gap-3">
                <BrainCircuit className="h-7 w-7 text-blue-600" />
                Agent Inbox
              </h1>
              <p className="text-sm text-muted-foreground mt-1">
                Review pending Google Calendar proposals, create them, and close open tasks.
              </p>
            </div>
          </div>
          <Button variant="outline" onClick={runNow} disabled={isRunning || status?.is_running}>
            <RefreshCw className={`h-4 w-4 ${(isRunning || status?.is_running) ? "animate-spin" : ""}`} />
            {isRunning || status?.is_running ? "Running..." : "Run now"}
          </Button>
        </div>
      </div>

      <div className="max-w-6xl mx-auto p-8 space-y-6">
        {isLoading ? (
          <div className="rounded-lg border border-border bg-card p-6">Loading agent inbox...</div>
        ) : (
          <>
            {!status?.api_key_configured && (
              <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800 flex items-start gap-2">
                <AlertCircle className="h-4 w-4 mt-0.5" />
                <span>Save a Gemini API key in Settings before the agent can analyze meetings.</span>
              </div>
            )}

            {status && !status.settings.enabled && (
              <div className="rounded-md border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-800 flex items-start gap-2">
                <AlertCircle className="h-4 w-4 mt-0.5" />
                <span>Background heartbeats are off. “Run now” will still analyze your latest meetings, but automatic suggestions will not appear until the agent is enabled in Settings.</span>
              </div>
            )}

            {status && !status.settings.calendar_proposals_enabled && (
              <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800 flex items-start gap-2">
                <AlertCircle className="h-4 w-4 mt-0.5" />
                <span>Enable Calendar proposals in Settings to have the agent draft Google Calendar events here.</span>
              </div>
            )}

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border border-border bg-card p-4">
                <div className="text-sm text-muted-foreground">Pending calendar proposals</div>
                <div className="text-3xl font-semibold mt-2">{status?.pending_recommendations ?? recommendations.length}</div>
              </div>
              <div className="rounded-lg border border-border bg-card p-4">
                <div className="text-sm text-muted-foreground">Open tasks</div>
                <div className="text-3xl font-semibold mt-2">{status?.open_tasks ?? tasks.length}</div>
              </div>
              <div className="rounded-lg border border-border bg-card p-4">
                <div className="text-sm text-muted-foreground">Last successful heartbeat</div>
                <div className="text-sm font-medium mt-2">{formatDateTime(status?.last_success_at)}</div>
              </div>
            </div>

            <div className="grid gap-6 lg:grid-cols-[1.35fr,0.95fr]">
              <div className="rounded-lg border border-border bg-card p-5 space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-semibold">Calendar proposals</h2>
                  <span className="text-sm text-muted-foreground">{recommendations.length} pending</span>
                </div>

                {recommendations.length === 0 ? (
                  <div className="rounded-md border border-border px-4 py-3 text-sm text-muted-foreground">
                    No pending Google Calendar proposals right now.
                  </div>
                ) : (
                  recommendations.map((recommendation) => (
                    <div key={recommendation.id} className="rounded-lg border border-border px-4 py-4 space-y-3">
                      <div className="flex items-start justify-between gap-4">
                        <div>
                          <div className="font-medium text-foreground">{recommendation.title}</div>
                          <div className="text-sm text-muted-foreground mt-1">{recommendation.body}</div>
                        </div>
                        <div className="rounded-full bg-secondary px-2 py-1 text-xs font-medium text-secondary-foreground">
                          {Math.round(recommendation.confidence * 100)}%
                        </div>
                      </div>

                      <div className="text-sm text-muted-foreground">{recommendation.rationale}</div>

                      {recommendation.recommendation_type === "calendar_event_draft" && recommendation.payload && (
                        <div className="rounded-md border border-border bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
                          <div className="flex items-center gap-2 font-medium text-foreground">
                            <CalendarPlus className="h-4 w-4" />
                            Draft event
                          </div>
                          <div className="mt-1">{recommendation.payload.title}</div>
                          <div>{formatDateTime(recommendation.payload.start_at)} to {formatDateTime(recommendation.payload.end_at)}</div>
                        </div>
                      )}

                      {recommendation.error && (
                        <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
                          {recommendation.error}
                        </div>
                      )}

                      <div className="flex flex-wrap gap-2">
                        <Button
                          variant="blue"
                          onClick={() => acceptRecommendation(recommendation)}
                          disabled={busyRecommendationId === recommendation.id}
                        >
                          {busyRecommendationId === recommendation.id ? "Creating..." : "Create in Google Calendar"}
                        </Button>
                        <Button
                          variant="outline"
                          onClick={() => dismissRecommendation(recommendation.id)}
                          disabled={busyRecommendationId === recommendation.id}
                        >
                          Dismiss
                        </Button>
                        {recommendation.source_meeting_id && (
                          <Button
                            variant="ghost"
                            onClick={() => router.push(`/meeting-details?id=${recommendation.source_meeting_id}`)}
                          >
                            View meeting
                          </Button>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>

              <div className="rounded-lg border border-border bg-card p-5 space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-semibold">Open tasks</h2>
                  <span className="text-sm text-muted-foreground">{tasks.length}</span>
                </div>

                {tasks.length === 0 ? (
                  <div className="rounded-md border border-border px-4 py-3 text-sm text-muted-foreground">
                    No open tasks right now.
                  </div>
                ) : (
                  tasks.map((task) => (
                    <div key={task.id} className="rounded-lg border border-border px-4 py-4 space-y-3">
                      <div>
                        <div className="font-medium text-foreground">{task.title}</div>
                        <div className="text-sm text-muted-foreground mt-1">{task.body}</div>
                      </div>
                      <div className="text-xs text-muted-foreground">
                        Priority: {task.priority} · Due: {formatDateTime(task.due_at)}
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Button
                          variant="outline"
                          onClick={() => completeTask(task.id)}
                          disabled={busyTaskId === task.id}
                        >
                          <CheckCircle2 className="h-4 w-4" />
                          {busyTaskId === task.id ? "Updating..." : "Complete"}
                        </Button>
                        {task.source_meeting_id && (
                          <Button
                            variant="ghost"
                            onClick={() => router.push(`/meeting-details?id=${task.source_meeting_id}`)}
                          >
                            View meeting
                          </Button>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>

            {status?.last_error && (
              <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 flex items-start gap-2">
                <XCircle className="h-4 w-4 mt-0.5" />
                <span>{status.last_error}</span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
