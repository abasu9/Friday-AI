import { invoke } from '@tauri-apps/api/core';

export interface AgentSettingsPayload {
  enabled: boolean;
  provider: string;
  model: string;
  notifications_enabled: boolean;
  calendar_proposals_enabled: boolean;
  heartbeat_interval_minutes: number;
}

export interface AgentStatusResponse {
  settings: AgentSettingsPayload;
  api_key_configured: boolean;
  calendar_connected: boolean;
  calendar_can_write: boolean;
  is_running: boolean;
  last_run_at: string | null;
  last_success_at: string | null;
  last_error: string | null;
  pending_recommendations: number;
  open_tasks: number;
}

export interface AgentMemoryItem {
  id: string;
  memory_type: string;
  title: string;
  body: string;
  source_meeting_id: string | null;
  source_calendar_event_id: string | null;
  subject_key: string;
  confidence: number;
  status: string;
  first_seen_at: string;
  last_seen_at: string;
}

export interface AgentTask {
  id: string;
  title: string;
  body: string;
  source_meeting_id: string | null;
  source_memory_item_id: string | null;
  owner_kind: string;
  due_at: string | null;
  priority: string;
  status: string;
  last_suggested_at: string;
}

export interface CreatedCalendarEventSummary {
  provider_event_id: string;
  title: string;
  start_at: string;
  end_at: string;
  html_link: string | null;
}

export interface AgentRecommendation {
  id: string;
  recommendation_type: string;
  title: string;
  body: string;
  rationale: string;
  confidence: number;
  source_meeting_id: string | null;
  source_calendar_event_id: string | null;
  task_id: string | null;
  payload: Record<string, any> | null;
  status: string;
  surfaced_at: string;
  acted_at: string | null;
  error: string | null;
}

export interface AgentMeetingContextResponse {
  memory_items: AgentMemoryItem[];
  tasks: AgentTask[];
  recommendations: AgentRecommendation[];
}

export interface AgentRecommendationActionResponse {
  recommendation: AgentRecommendation;
  created_calendar_event: CreatedCalendarEventSummary | null;
}

class AgentService {
  async getStatus(): Promise<AgentStatusResponse> {
    return invoke<AgentStatusResponse>('agent_get_status');
  }

  async getSettings(): Promise<AgentSettingsPayload> {
    return invoke<AgentSettingsPayload>('agent_get_settings');
  }

  async setSettings(settings: AgentSettingsPayload): Promise<AgentStatusResponse> {
    return invoke<AgentStatusResponse>('agent_set_settings', { settings });
  }

  async saveGeminiApiKey(apiKey: string): Promise<void> {
    return invoke('agent_save_gemini_api_key', { apiKey });
  }

  async clearGeminiApiKey(): Promise<void> {
    return invoke('agent_clear_gemini_api_key');
  }

  async runHeartbeatNow(): Promise<AgentStatusResponse> {
    return invoke<AgentStatusResponse>('agent_run_heartbeat_now');
  }

  async listRecommendations(status?: string): Promise<AgentRecommendation[]> {
    return invoke<AgentRecommendation[]>('agent_list_recommendations', { status: status ?? null });
  }

  async acceptRecommendation(recommendationId: string): Promise<AgentRecommendationActionResponse> {
    return invoke<AgentRecommendationActionResponse>('agent_accept_recommendation', { recommendationId });
  }

  async dismissRecommendation(recommendationId: string): Promise<AgentRecommendation> {
    return invoke<AgentRecommendation>('agent_dismiss_recommendation', { recommendationId });
  }

  async listMemory(limit = 25): Promise<AgentMemoryItem[]> {
    return invoke<AgentMemoryItem[]>('agent_list_memory', { limit });
  }

  async getMeetingContext(meetingId: string): Promise<AgentMeetingContextResponse> {
    return invoke<AgentMeetingContextResponse>('agent_get_meeting_context', { meetingId });
  }

  async listTasks(status?: string): Promise<AgentTask[]> {
    return invoke<AgentTask[]>('agent_list_tasks', { status: status ?? null });
  }

  async updateTaskStatus(taskId: string, status: string): Promise<AgentTask[]> {
    return invoke<AgentTask[]>('agent_update_task_status', { taskId, status });
  }
}

export const agentService = new AgentService();
