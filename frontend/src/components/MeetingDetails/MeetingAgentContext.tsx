"use client";

import { useEffect, useState } from "react";
import { CheckCircle2, Clock3, Sparkles } from "lucide-react";
import { toast } from "sonner";
import { agentService, AgentMeetingContextResponse } from "@/services/agentService";
import { Button } from "@/components/ui/button";

interface MeetingAgentContextProps {
  meetingId: string;
}

function formatDateTime(value: string | null | undefined): string {
  if (!value) return "No due date";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

export function MeetingAgentContext({ meetingId }: MeetingAgentContextProps) {
  const [context, setContext] = useState<AgentMeetingContextResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isUpdatingTaskId, setIsUpdatingTaskId] = useState<string | null>(null);

  const loadContext = async () => {
    try {
      const nextContext = await agentService.getMeetingContext(meetingId);
      setContext(nextContext);
    } catch (error) {
      console.error("Failed to load meeting agent context:", error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    setIsLoading(true);
    void loadContext();
  }, [meetingId]);

  const markTaskCompleted = async (taskId: string) => {
    setIsUpdatingTaskId(taskId);
    try {
      await agentService.updateTaskStatus(taskId, "completed");
      await loadContext();
      toast.success("Task marked complete");
    } catch (error) {
      console.error("Failed to update task:", error);
      toast.error("Failed to update task", { description: String(error) });
    } finally {
      setIsUpdatingTaskId(null);
    }
  };

  if (isLoading) {
    return (
      <div className="mb-4 rounded-lg border border-border bg-card px-4 py-3 text-sm text-muted-foreground">
        Loading agent memory...
      </div>
    );
  }

  if (!context || (!context.memory_items.length && !context.tasks.length && !context.recommendations.length)) {
    return null;
  }

  return (
    <div className="mb-4 rounded-lg border border-border bg-card px-4 py-4 space-y-4">
      <div className="flex items-center gap-2 text-sm font-medium text-foreground">
        <Sparkles className="h-4 w-4" />
        Agent Context
      </div>

      {context.memory_items.length > 0 && (
        <div className="space-y-2">
          <div className="text-sm font-medium text-foreground">Memory</div>
          {context.memory_items.slice(0, 4).map((item) => (
            <div key={item.id} className="rounded-md border border-border px-3 py-2">
              <div className="text-sm font-medium text-foreground">{item.title}</div>
              <div className="text-sm text-muted-foreground">{item.body}</div>
            </div>
          ))}
        </div>
      )}

      {context.tasks.length > 0 && (
        <div className="space-y-2">
          <div className="text-sm font-medium text-foreground">Tasks</div>
          {context.tasks.slice(0, 4).map((task) => (
            <div key={task.id} className="rounded-md border border-border px-3 py-2 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <div className="text-sm font-medium text-foreground">{task.title}</div>
                <div className="text-sm text-muted-foreground">{task.body}</div>
                <div className="text-xs text-muted-foreground mt-1">
                  <Clock3 className="inline h-3.5 w-3.5 mr-1" />
                  {formatDateTime(task.due_at)}
                </div>
              </div>
              {task.status !== "completed" && (
                <Button
                  variant="outline"
                  size="sm"
                  disabled={isUpdatingTaskId === task.id}
                  onClick={() => markTaskCompleted(task.id)}
                >
                  <CheckCircle2 className="h-4 w-4" />
                  {isUpdatingTaskId === task.id ? "Updating..." : "Complete"}
                </Button>
              )}
            </div>
          ))}
        </div>
      )}

      {context.recommendations.length > 0 && (
        <div className="space-y-2">
          <div className="text-sm font-medium text-foreground">Recent recommendations</div>
          {context.recommendations.slice(0, 3).map((recommendation) => (
            <div key={recommendation.id} className="rounded-md border border-border px-3 py-2">
              <div className="text-sm font-medium text-foreground">{recommendation.title}</div>
              <div className="text-sm text-muted-foreground">{recommendation.body}</div>
              <div className="text-xs text-muted-foreground mt-1">{recommendation.rationale}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
