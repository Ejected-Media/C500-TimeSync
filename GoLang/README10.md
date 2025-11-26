Here is the comprehensive README.md for the C500 TimeSync module. You can save this directly into your project repository to guide the development process.
C500 TimeSync: Context-Aware Scheduling System
Project: Classroom 500 (C500)
Module: TimeSync & Concierge
Status: Architecture Specification

📖 Overview
C500 TimeSync is a specialized scheduling engine designed to solve the "Timezone Math" struggle and the "Invisible Wall" of remote communication. Unlike standard calendars, this system calculates scheduling based on UTC-Anchors and enriches every interaction with Environmental Context (Weather, Time of Day, Sunlight) to build empathy between mentors and students.
🏗️ System Architecture
 * Core Backend: GoLang (Google Cloud Run)
 * Database: Firebase Firestore (NoSQL)
 * User Interface: Discord Bot (Interactions/Ephemeral Messages)
 * Admin Interface: Kotlin Android App (Remote Control)
 * Context APIs: Open-Meteo (Weather), go-sunrise (Astronomy)
 * Job Scheduling: Google Cloud Scheduler
🌍 The "Global Anchor" Strategy
We do not use standard timezone offsets. We use IANA Location Anchors to ensure accurate time conversion regardless of Daylight Saving Time (DST) changes.
Primary Reference Map:

| Friendly Name | IANA Code | Role |
| :--- | :--- | :--- |
| Phoenix (HQ) | America/Phoenix | Mentor / Admin (UTC-7, No DST) |
| Honolulu | Pacific/Honolulu | Reference (UTC-10) |
| Sweden | Europe/Stockholm | Reference (CET/CEST) |
| Riyadh / Bahrain | Asia/Riyadh | Middle East Anchor (UTC+3) |
| Bangkok | Asia/Bangkok | SE Asia Anchor (UTC+7) |
| Sydney | Australia/Sydney | APAC Anchor (UTC+10/11) |

> Golden Rule: Store all time in Database as UTC. Calculate logic in Go. Display only Local Time.
> 
⚡ Key Features
1. The "Concierge" Booking Flow
 * Interface: Discord Slash Command /book-session.
 * Mechanism:
   * User requests slots.
   * Backend fetches OPEN slots from Firestore.
   * Backend converts UTC slots to User's mapped timezone (e.g., Riyadh).
   * Bot responds with an Ephemeral Dropdown Menu (Privacy first).
   * Selection triggers a Firestore Transaction to prevent double-booking.

2. The Context Engine ("Empathy Algorithm")
Every booking captures a snapshot of the physical environment for both parties.
 * Astronomy Layer: Calculates Sun Phase (Day/Night) using Lat/Long.
 * Meteorology Layer: Fetches Temperature and Weather Codes via Open-Meteo.
 * Severity Logic:
   * Level 0 (Normal): Clear, Cloudy, Mild.
   * Level 1 (Uncomfortable): Heavy Rain, Heat (>35°C), Freezing (<0°C).
   * Level 2 (Extreme/Dangerous): Thunderstorms, Snowstorms, Heat (>40°C).

3. Smart Reminders
 * Trigger: Cloud Scheduler hits /tasks/check-reminders every 15 mins.
 * Logic: Finds sessions starting in 30 minutes.
 * Live Check: Re-scans the Mentor's local weather.
 * Output: If severity is High, the reminder includes a warning:
   > "⚠️ Environment Alert: Your mentor is currently in a Severe Thunderstorm. Connection jitter is possible."
   > 

4. Emergency Protocol ("Red Button")
 * Command: /emergency-cancel [reason] (Admin Only).
 * Function:
   * Finds the next immediate BOOKED slot.
   * Updates status to CANCELLED.
   * Fetches live weather to validate the reason (e.g., "Power Outage" + "Heavy Storm").
   * Sends a formatted DM to the student handling the apology.
💾 Database Schema (Firestore)

```
Collection: availability_slots
{
  "doc_id": "slot_uuid_v4",
  "instructor_id": "discord_user_id",
  "start_time_utc": "2025-11-27T22:00:00Z", // TIMESTAMP
  "duration_minutes": 30,
  "status": "BOOKED", // OPEN, BOOKED, CANCELLED
  "student_id": "discord_user_id",
  
  // The Context Snapshot (Frozen at moment of booking)
  "context_snapshot": {
    "mentor": {
      "location": "Phoenix, AZ",
      "time_str": "3:00 PM",
      "weather": "Dangerous Heat",
      "temp_f": 112
    },
    "student": {
      "location": "Stockholm, SE",
      "time_str": "12:00 AM",
      "weather": "Clear",
      "temp_f": 45
    }
  }
}
```

🛠️ Development Roadmap
 * Phase 1 (Go): Implement time.LoadLocation logic and the "Reference Map" printer.
 * Phase 2 (Discord): Build the /book-session command with Dropdown components.
 * Phase 3 (Firestore): Implement the atomic booking transaction.
 * Phase 4 (Context): Integrate go-sunrise and Open-Meteo API.
 * Phase 5 (Ops): Deploy to Cloud Run and set up Cloud Scheduler.

📝 Note on Weather API
This project uses Open-Meteo (Free for non-commercial use, no API key required).
 * Endpoint: https://api.open-meteo.com/v1/forecast?latitude=...&longitude=...&current_weather=true
 * Attribution: Ensure attribution to Open-Meteo is included in the project credits.

Would you like me to start guiding you through the first step: Setting up the GoLang project structure and the go.mod file?
