'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { Calendar, Clock, Play, Users, Video } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  calendarService,
  UpcomingCalendarEvent,
} from '@/services/calendarService';

interface UpcomingMeetingsProps {
  onStartRecording: (meetingTitle: string) => void;
  isRecording: boolean;
}

function formatTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
}

function formatRelative(isoString: string): string {
  const now = Date.now();
  const start = new Date(isoString).getTime();
  const diffMin = Math.round((start - now) / 60000);
  if (diffMin < -5) return 'Started';
  if (diffMin <= 0) return 'Now';
  if (diffMin < 60) return `In ${diffMin}m`;
  const hours = Math.floor(diffMin / 60);
  const mins = diffMin % 60;
  return mins > 0 ? `In ${hours}h ${mins}m` : `In ${hours}h`;
}

function isHappeningSoon(event: UpcomingCalendarEvent): boolean {
  const now = Date.now();
  const start = new Date(event.start_at).getTime();
  const end = new Date(event.end_at).getTime();
  return now >= start - 5 * 60000 && now <= end;
}

export function UpcomingMeetings({ onStartRecording, isRecording }: UpcomingMeetingsProps) {
  const [events, setEvents] = useState<UpcomingCalendarEvent[]>([]);
  const [connected, setConnected] = useState(false);
  const [showingCachedEvents, setShowingCachedEvents] = useState(false);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const status = await calendarService.getStatus();
      const account = status.account;
      const hasReadableCalendar =
        account !== null && account.connection_status !== 'disconnected';
      setConnected(hasReadableCalendar);
      setShowingCachedEvents(account?.connection_status === 'error');
      if (!hasReadableCalendar) {
        setEvents([]);
        return;
      }
      const upcoming = await calendarService.listUpcoming();
      setEvents(upcoming);
    } catch (err) {
      console.error('Failed to load upcoming events:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 60000);
    return () => clearInterval(interval);
  }, [refresh]);

  if (loading || !connected || events.length === 0) return null;

  return (
    <div className="w-full space-y-2">
      <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
        <Calendar className="h-4 w-4" />
        Upcoming Meetings
        {showingCachedEvents && (
          <span className="rounded-full bg-amber-100 px-2 py-0.5 text-[11px] font-medium text-amber-700">
            Cached
          </span>
        )}
      </div>
      <div className="space-y-2">
        {events.map((event) => {
          const soon = isHappeningSoon(event);
          const attendeeCount = event.attendees?.length ?? 0;

          return (
            <div
              key={event.provider_event_id}
              className={`flex items-center justify-between rounded-lg border px-4 py-3 transition-colors ${
                soon
                  ? 'border-blue-200 bg-blue-50/50'
                  : 'border-border bg-card'
              }`}
            >
              <div className="flex-1 min-w-0 space-y-0.5">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-foreground truncate">
                    {event.title}
                  </span>
                  {soon && (
                    <span className="flex-shrink-0 rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                      {formatRelative(event.start_at)}
                    </span>
                  )}
                  {!soon && (
                    <span className="flex-shrink-0 text-xs text-muted-foreground">
                      {formatRelative(event.start_at)}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {formatTime(event.start_at)} – {formatTime(event.end_at)}
                  </span>
                  {attendeeCount > 0 && (
                    <span className="flex items-center gap-1">
                      <Users className="h-3 w-3" />
                      {attendeeCount}
                    </span>
                  )}
                  {event.conference_url && (
                    <a
                      href={event.conference_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-1 text-blue-600 hover:underline"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <Video className="h-3 w-3" />
                      Join
                    </a>
                  )}
                </div>
              </div>
              <Button
                variant={soon ? 'blue' : 'outline'}
                size="sm"
                disabled={isRecording}
                onClick={() => onStartRecording(event.title)}
                className="ml-3 flex-shrink-0"
              >
                <Play className="h-3.5 w-3.5" />
                {soon ? 'Start' : 'Record'}
              </Button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
