import { invoke } from '@tauri-apps/api/core';

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

class CalendarService {
  async getStatus(): Promise<CalendarStatusResponse> {
    return invoke<CalendarStatusResponse>('calendar_get_status');
  }

  async listUpcoming(): Promise<UpcomingCalendarEvent[]> {
    return invoke<UpcomingCalendarEvent[]>('calendar_list_upcoming');
  }

  async connectGoogle(writeAccess = false): Promise<CalendarStatusResponse> {
    return invoke<CalendarStatusResponse>('calendar_connect_google', {
      writeAccess,
    });
  }

  async upgradeGoogleAccess(): Promise<CalendarStatusResponse> {
    return invoke<CalendarStatusResponse>('calendar_upgrade_google_access');
  }

  async disconnectGoogle(): Promise<CalendarStatusResponse> {
    return invoke<CalendarStatusResponse>('calendar_disconnect_google');
  }

  async syncNow(): Promise<CalendarSyncResult> {
    return invoke<CalendarSyncResult>('calendar_sync_now');
  }

  async getMeetingLink(meetingId: string): Promise<LinkedCalendarEvent | null> {
    return invoke<LinkedCalendarEvent | null>('calendar_get_meeting_link', {
      meetingId,
    });
  }

  async getLinkCandidates(meetingId: string): Promise<CalendarLinkCandidate[]> {
    return invoke<CalendarLinkCandidate[]>('calendar_get_link_candidates', {
      meetingId,
    });
  }

  async setMeetingLink(
    meetingId: string,
    providerEventId: string
  ): Promise<LinkedCalendarEvent | null> {
    return invoke<LinkedCalendarEvent | null>('calendar_set_meeting_link', {
      meetingId,
      providerEventId,
    });
  }

  async clearMeetingLink(meetingId: string): Promise<void> {
    return invoke('calendar_clear_meeting_link', { meetingId });
  }
}

export const calendarService = new CalendarService();
