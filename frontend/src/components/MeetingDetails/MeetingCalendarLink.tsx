"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { CalendarClock, Link2, RefreshCw, Unlink } from "lucide-react";
import { toast } from "sonner";
import {
  calendarService,
  CalendarLinkCandidate,
  LinkedCalendarEvent,
} from "@/services/calendarService";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface MeetingCalendarLinkProps {
  meetingId: string;
}

function formatEventWindow(startAt: string, endAt: string): string {
  const start = new Date(startAt);
  const end = new Date(endAt);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
    return `${startAt} - ${endAt}`;
  }

  return `${start.toLocaleString()} - ${end.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  })}`;
}

export function MeetingCalendarLink({ meetingId }: MeetingCalendarLinkProps) {
  const [linkedEvent, setLinkedEvent] = useState<LinkedCalendarEvent | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const [candidateDialogOpen, setCandidateDialogOpen] = useState(false);
  const [candidates, setCandidates] = useState<CalendarLinkCandidate[]>([]);
  const [isLoadingCandidates, setIsLoadingCandidates] = useState(false);
  const [isApplyingCandidate, setIsApplyingCandidate] = useState<string | null>(null);

  const attendeeSummary = useMemo(() => {
    if (!linkedEvent?.attendees?.length) return null;
    return linkedEvent.attendees
      .slice(0, 3)
      .map((attendee) => attendee.display_name || attendee.email)
      .filter(Boolean)
      .join(", ");
  }, [linkedEvent]);

  const loadLinkedEvent = useCallback(async () => {
    try {
      const nextLink = await calendarService.getMeetingLink(meetingId);
      setLinkedEvent(nextLink);
    } catch (error) {
      console.error("Failed to load meeting calendar link:", error);
    } finally {
      setIsLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    setIsLoading(true);
    loadLinkedEvent();
  }, [loadLinkedEvent]);

  const openCandidateDialog = async () => {
    setCandidateDialogOpen(true);
    setIsLoadingCandidates(true);
    try {
      const nextCandidates = await calendarService.getLinkCandidates(meetingId);
      setCandidates(nextCandidates);
    } catch (error) {
      console.error("Failed to load calendar link candidates:", error);
      toast.error("Failed to load calendar events", {
        description: String(error),
      });
    } finally {
      setIsLoadingCandidates(false);
    }
  };

  const refreshLink = async () => {
    setIsRefreshing(true);
    try {
      await loadLinkedEvent();
    } finally {
      setIsRefreshing(false);
    }
  };

  const clearLink = async () => {
    setIsClearing(true);
    try {
      await calendarService.clearMeetingLink(meetingId);
      setLinkedEvent(null);
      toast.success("Calendar link cleared");
    } catch (error) {
      console.error("Failed to clear calendar link:", error);
      toast.error("Failed to clear calendar link", {
        description: String(error),
      });
    } finally {
      setIsClearing(false);
    }
  };

  const selectCandidate = async (candidate: CalendarLinkCandidate) => {
    setIsApplyingCandidate(candidate.provider_event_id);
    try {
      const nextLink = await calendarService.setMeetingLink(
        meetingId,
        candidate.provider_event_id
      );
      setLinkedEvent(nextLink);
      setCandidateDialogOpen(false);
      toast.success("Calendar event linked");
    } catch (error) {
      console.error("Failed to link calendar event:", error);
      toast.error("Failed to link calendar event", {
        description: String(error),
      });
    } finally {
      setIsApplyingCandidate(null);
    }
  };

  if (isLoading) {
    return (
      <div className="mb-4 rounded-lg border border-border bg-card px-4 py-3 text-sm text-muted-foreground">
        Loading calendar context...
      </div>
    );
  }

  return (
    <>
      <div className="mb-4 rounded-lg border border-border bg-card px-4 py-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-sm font-medium text-foreground">
              <CalendarClock className="h-4 w-4" />
              Calendar Event
            </div>
            {linkedEvent ? (
              <>
                <div className="text-base font-semibold text-foreground">
                  {linkedEvent.title}
                </div>
                <div className="text-sm text-muted-foreground">
                  {formatEventWindow(linkedEvent.start_at, linkedEvent.end_at)}
                </div>
                {linkedEvent.reason && (
                  <div className="text-sm text-muted-foreground">
                    {linkedEvent.reason}
                  </div>
                )}
                {(linkedEvent.organizer_name || linkedEvent.organizer_email) && (
                  <div className="text-sm text-muted-foreground">
                    Organizer: {linkedEvent.organizer_name || linkedEvent.organizer_email}
                  </div>
                )}
                {attendeeSummary && (
                  <div className="text-sm text-muted-foreground">
                    Attendees: {attendeeSummary}
                  </div>
                )}
              </>
            ) : (
              <div className="text-sm text-muted-foreground">
                No calendar event is linked to this meeting yet.
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={refreshLink} disabled={isRefreshing}>
              <RefreshCw className={`h-4 w-4 ${isRefreshing ? "animate-spin" : ""}`} />
              Refresh
            </Button>
            <Button variant="outline" size="sm" onClick={openCandidateDialog}>
              <Link2 className="h-4 w-4" />
              {linkedEvent ? "Change link" : "Find event"}
            </Button>
            {linkedEvent && (
              <Button
                variant="ghost"
                size="sm"
                onClick={clearLink}
                disabled={isClearing}
              >
                <Unlink className="h-4 w-4" />
                {isClearing ? "Clearing..." : "Clear"}
              </Button>
            )}
          </div>
        </div>
      </div>

      <Dialog open={candidateDialogOpen} onOpenChange={setCandidateDialogOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Link meeting to a calendar event</DialogTitle>
            <DialogDescription>
              Friday suggests nearby synced Google Calendar events so you can confirm or override the link.
            </DialogDescription>
          </DialogHeader>

          <div className="max-h-[420px] overflow-y-auto space-y-3">
            {isLoadingCandidates ? (
              <div className="rounded-md border border-border px-4 py-3 text-sm text-muted-foreground">
                Loading nearby calendar events...
              </div>
            ) : candidates.length === 0 ? (
              <div className="rounded-md border border-border px-4 py-3 text-sm text-muted-foreground">
                No nearby synced Google Calendar events were found for this meeting.
              </div>
            ) : (
              candidates.map((candidate) => (
                <button
                  key={candidate.provider_event_id}
                  type="button"
                  onClick={() => selectCandidate(candidate)}
                  disabled={isApplyingCandidate !== null}
                  className="w-full rounded-lg border border-border bg-background px-4 py-3 text-left transition-colors hover:bg-accent disabled:opacity-60"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-1">
                      <div className="font-medium text-foreground">{candidate.title}</div>
                      <div className="text-sm text-muted-foreground">
                        {formatEventWindow(candidate.start_at, candidate.end_at)}
                      </div>
                      <div className="text-sm text-muted-foreground">{candidate.reason}</div>
                      {(candidate.organizer_name || candidate.organizer_email) && (
                        <div className="text-sm text-muted-foreground">
                          Organizer: {candidate.organizer_name || candidate.organizer_email}
                        </div>
                      )}
                    </div>
                    <div className="rounded-full bg-secondary px-2 py-1 text-xs font-medium text-secondary-foreground">
                      {Math.round(candidate.confidence * 100)}%
                    </div>
                  </div>
                </button>
              ))
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setCandidateDialogOpen(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
