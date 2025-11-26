This is the final safety net. When the power goes out or an emergency happens, you don't have time to look up database IDs. You need a "Red Button" protocol.
We will build an Admin Command (and later, a big red button in your Kotlin app) that cancels the next upcoming session and handles the apology for you.

1. The Cancellation Flow
The flow is designed for speed:
 * Admin triggers /emergency-cancel (or presses the button in the app).
 * System identifies the immediate next booked session.
 * System updates Firestore status to CANCELLED.
 * System drafts a sympathetic message to the student using the Context Engine (e.g., "Due to severe weather...").

2. The GoLang Implementation (The "Red Button" Logic)
This function finds the single nearest booked slot and nukes it.
Command: /emergency-cancel [reason]
 * reason (optional): Defaults to "Unforeseen Emergency".

```
<!-- end list -->
func HandleEmergencyCancel(s *discordgo.Session, i *discordgo.InteractionCreate) {
    // 1. SECURITY: Only allow YOU (The Mentor) to run this
    if i.Member.User.ID != "YOUR_ADMIN_DISCORD_ID" {
        return // Silent fail or error message
    }

    // 2. GET REASON: Did you type one? Or is it default?
    reason := "an unforeseen emergency"
    options := i.ApplicationCommandData().Options
    if len(options) > 0 {
        reason = options[0].StringValue()
    }

    ctx := context.Background()
    now := time.Now().UTC()

    // 3. FIND NEXT SESSION: Query for the very first 'BOOKED' slot in the future
    iter := firestoreClient.Collection("availability_slots").
        Where("status", "==", "BOOKED").
        Where("start_time_utc", ">=", now).
        OrderBy("start_time_utc", firestore.Asc). // Get the soonest one
        Limit(1).
        Documents(ctx)
        
    doc, err := iter.Next()
    if err != nil {
        // No upcoming sessions found
        respond(s, i, "✅ You have no upcoming sessions to cancel.")
        return
    }

    var booking BookingRecord
    doc.DataTo(&booking)

    // 4. UPDATE DATABASE: Mark as Cancelled
    doc.Ref.Update(ctx, []firestore.Update{
        {Path: "status", Value: "CANCELLED"},
        {Path: "cancellation_reason", Value: reason},
    })

    // 5. FETCH LIVE CONTEXT (To validate the emergency)
    // Checking Phoenix weather to add credibility to the cancellation
    weather := FetchWeather(33.44, -112.07) 
    analysis := AnalyzeConditions(weather.TempC, weather.Code)

    // 6. NOTIFY STUDENT
    SendCancellationMessage(s, booking.StudentDiscordID, reason, analysis)

    respond(s, i, fmt.Sprintf("🚨 **CANCELLATION EXECUTED.**\nSession with <@%s> has been cancelled.", booking.StudentDiscordID))
}
```

3. The Student Experience (The Notification)
This is crucial. You want to save the relationship even though you are cancelling last minute.
The Go Function to build the message:

```
func SendCancellationMessage(s *discordgo.Session, studentID string, reason string, cond EmpathyLevel) {
    
    // Base Message
    title := "⚠️ Session Update: Cancellation"
    desc := fmt.Sprintf("Your mentor has had to cancel the upcoming session due to **%s**.", reason)

    // The "Empathy" Context
    // If the weather is bad, we explicitly blame the weather.
    if cond.Severity >= 1 {
        desc += fmt.Sprintf("\n\n**Current Status at Mentor HQ:**\n%s %s (Likely cause of disruption)", cond.AlertEmoji, cond.Description)
    }

    // Embed Construction
    embed := &discordgo.MessageEmbed{
        Title:       title,
        Description: desc,
        Color:       0xFF0000, // Red
        Footer:      &discordgo.MessageEmbedFooter{Text: "You will be credited for this session immediately."},
    }

    // Send DM
    channel, _ := s.UserChannelCreate(studentID)
    s.ChannelMessageSendEmbed(channel.ID, embed)
}
```

4. What the Student Sees
Scenario: You wake up sick (No weather issue)
> ⚠️ Session Update: Cancellation
> Your mentor has had to cancel the upcoming session due to a medical issue.
> You will be credited for this session immediately.
> 
Scenario: Power outage during a storm
> ⚠️ Session Update: Cancellation
> Your mentor has had to cancel the upcoming session due to a power outage.
> Current Status at Mentor HQ:
> ⛈️ Thunderstorm (Likely cause of disruption)
> You will be credited for this session immediately.
> 
Project Summary & Next Step
We have designed a complete, resilient scheduling ecosystem for Classroom 500 (C500):
 * Global Time Engine: Solves the "Timezone Math" struggle using Asia/Riyadh and Asia/Bangkok as anchors.
 * Context Aware: Uses "Empathy Levels" to display weather/environment data.
 * Discord Native: Booking happens entirely via DMs/Buttons.
 * Emergency Protocols: A "Red Button" to handle cancellations gracefully.
This concludes the backend architecture design phase.
Would you like me to generate a README.md file summarizing this entire technical specification so you can save it to your project folder, or are you ready to switch context to the Android App UI design?
