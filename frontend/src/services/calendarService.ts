const isTauri = (): boolean =>
  typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error("Google Calendar requires the Friday desktop app (Tauri runtime not available).");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export interface CalendarAccountSummary {
  email: string | null;
  connection_status: string;
  last_sync_at: string | null;
  last_error: string | null;
  scopes: string[];
}

export interface CalendarStatusResponse {
  client_configured: boolean;
  connected: boolean;
  can_write: boolean;
  syncing: boolean;
  account: CalendarAccountSummary | null;
}

export interface CalendarSyncResult {
  synced_events: number;
  synced_at: string;
}

export interface CalendarAttendeeSummary {
  email: string | null;
  display_name: string | null;
  response_status: string | null;
}

export interface LinkedCalendarEvent {
  provider_event_id: string;
  title: string;
  description: string | null;
  organizer_email: string | null;
  organizer_name: string | null;
  attendees: CalendarAttendeeSummary[];
  start_at: string;
  end_at: string;
  timezone: string | null;
  conference_url: string | null;
  status: string;
  html_link: string | null;
  confidence: number;
  link_method: string;
  reason: string | null;
  linked_at: string;
}

export interface CalendarLinkCandidate {
  provider_event_id: string;
  title: string;
  start_at: string;
  end_at: string;
  organizer_email: string | null;
  organizer_name: string | null;
  conference_url: string | null;
  html_link: string | null;
  confidence: number;
  reason: string;
}

export interface UpcomingCalendarEvent {
  provider_event_id: string;
  title: string;
  start_at: string;
  end_at: string;
  organizer_email: string | null;
  organizer_name: string | null;
  attendees: CalendarAttendeeSummary[];
  conference_url: string | null;
  html_link: string | null;
}

const DEFAULT_STATUS: CalendarStatusResponse = {
  client_configured: false,
  connected: false,
  can_write: false,
  syncing: false,
  account: null,
};

class CalendarService {
  async getStatus(): Promise<CalendarStatusResponse> {
    if (!isTauri()) return DEFAULT_STATUS;
    return tauriInvoke<CalendarStatusResponse>('calendar_get_status');
  }

  async listUpcoming(): Promise<UpcomingCalendarEvent[]> {
    if (!isTauri()) return [];
    return tauriInvoke<UpcomingCalendarEvent[]>('calendar_list_upcoming');
  }

  async connectGoogle(writeAccess = false): Promise<CalendarStatusResponse> {
    return tauriInvoke<CalendarStatusResponse>('calendar_connect_google', {
      writeAccess,
    });
  }

  async upgradeGoogleAccess(): Promise<CalendarStatusResponse> {
    return tauriInvoke<CalendarStatusResponse>('calendar_upgrade_google_access');
  }

  async disconnectGoogle(): Promise<CalendarStatusResponse> {
    return tauriInvoke<CalendarStatusResponse>('calendar_disconnect_google');
  }

  async syncNow(): Promise<CalendarSyncResult> {
    return tauriInvoke<CalendarSyncResult>('calendar_sync_now');
  }

  async getMeetingLink(meetingId: string): Promise<LinkedCalendarEvent | null> {
    return tauriInvoke<LinkedCalendarEvent | null>('calendar_get_meeting_link', {
      meetingId,
    });
  }

  async getLinkCandidates(meetingId: string): Promise<CalendarLinkCandidate[]> {
    return tauriInvoke<CalendarLinkCandidate[]>('calendar_get_link_candidates', {
      meetingId,
    });
  }

  async setMeetingLink(
    meetingId: string,
    providerEventId: string
  ): Promise<LinkedCalendarEvent | null> {
    return tauriInvoke<LinkedCalendarEvent | null>('calendar_set_meeting_link', {
      meetingId,
      providerEventId,
    });
  }

  async clearMeetingLink(meetingId: string): Promise<void> {
    return tauriInvoke('calendar_clear_meeting_link', { meetingId });
  }
}

export const calendarService = new CalendarService();
