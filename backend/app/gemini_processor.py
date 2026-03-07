"""
Friday Gemini Processor
-----------------------
Takes raw meeting transcript text, sends it to Google Gemini 1.5 Pro,
and extracts a structured JSON object with summary, action items, and deadlines.
Saves the result to friday_state.json for downstream agent consumption.
"""

import json
import logging
import os
from pathlib import Path
from typing import Optional

from google import genai
from pydantic import BaseModel

logger = logging.getLogger(__name__)

FRIDAY_STATE_PATH = Path(__file__).resolve().parent.parent / "friday_state.json"

EXTRACTION_PROMPT = """\
You are Friday, an executive assistant AI. Analyze the following meeting transcript \
and extract structured information. Be precise and concise.

Return ONLY a valid JSON object with exactly this schema:

{{
  "summary": "<A concise 2-sentence summary of the meeting>",
  "action_items": [
    {{
      "task": "<What needs to be done>",
      "owner": "<Person responsible, or 'Unassigned' if unclear>",
      "estimated_minutes": <integer estimate of minutes to complete, or null if unclear>
    }}
  ],
  "deadlines": [
    {{
      "description": "<What is due>",
      "date": "<ISO 8601 date/datetime string, or natural language if exact date unclear>",
      "owner": "<Person responsible, or 'Unassigned'>"
    }}
  ]
}}

Rules:
- If no action items are found, return an empty array for "action_items".
- If no deadlines are mentioned, return an empty array for "deadlines".
- Do NOT wrap the JSON in markdown code fences. Return raw JSON only.

TRANSCRIPT:
---
{transcript}
---
"""


class FridayExtraction(BaseModel):
    """The structured output Friday produces from a meeting transcript."""
    summary: str
    action_items: list
    deadlines: list


class FridayExtractor:
    """Calls Google Gemini to extract structured meeting data from a transcript."""

    def __init__(self, api_key: Optional[str] = None):
        key = api_key or os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY")
        if not key:
            raise ValueError(
                "Gemini API key not found. Set GEMINI_API_KEY or GOOGLE_API_KEY env var, "
                "or pass api_key directly."
            )
        self.client = genai.Client(api_key=key)
        self.model = "gemini-2.0-flash"

    async def extract(self, transcript: str) -> dict:
        """
        Send transcript to Gemini and return the parsed Friday extraction dict.
        Also persists the result to friday_state.json.
        """
        prompt = EXTRACTION_PROMPT.format(transcript=transcript)

        logger.info("Sending transcript (%d chars) to Gemini model %s", len(transcript), self.model)

        response = self.client.models.generate_content(
            model=self.model,
            contents=prompt,
            config=genai.types.GenerateContentConfig(
                temperature=0.2,
                response_mime_type="application/json",
            ),
        )

        raw_text = response.text.strip()
        logger.debug("Gemini raw response: %s", raw_text[:500])

        try:
            result = json.loads(raw_text)
        except json.JSONDecodeError:
            logger.error("Gemini returned non-JSON response: %s", raw_text[:300])
            raise ValueError("Gemini did not return valid JSON. Raw response logged above.")

        validated = FridayExtraction(**result)
        result_dict = validated.model_dump()

        self._save_state(result_dict)
        logger.info("Friday extraction complete. %d action items, %d deadlines.",
                     len(result_dict["action_items"]), len(result_dict["deadlines"]))
        return result_dict

    def _save_state(self, data: dict) -> None:
        """Persist extraction to friday_state.json for the agent heartbeat."""
        try:
            FRIDAY_STATE_PATH.write_text(json.dumps(data, indent=2))
            logger.info("Saved friday_state.json to %s", FRIDAY_STATE_PATH)
        except Exception as e:
            logger.error("Failed to write friday_state.json: %s", e)
