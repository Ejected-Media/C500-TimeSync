package handlers

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"time"

	"cloud.google.com/go/firestore"
	"github.com/gin-gonic/gin"
	"google.golang.org/api/iterator"
)

// CheckRemindersHandler handles incoming HTTP POST requests from Google Cloud Scheduler.
func CheckRemindersHandler(client *firestore.Client, discordNotifier func(discordID string, message string) error) gin.HandlerFunc {
	return func(c *gin.Context) {
		ctx := c.Request.Context()
		now := time.Now().UTC()
		
		// Window: Looking for sessions starting between now and 20 minutes from now
		windowStart := now
		windowEnd := now.Add(20 * time.Minute)

		log.Printf("[ReminderJob] Scanning for sessions starting between %s and %s", 
			windowStart.Format(time.RFC3339), windowEnd.Format(time.RFC3339))

		// Query all mentors or use a root collection group query if you flatten your slots.
		// For a mentor subcollection structure, we iterate or use CollectionGroup.
		// Here we use a CollectionGroup query on "availability_slots" across all mentors for efficiency.
		query := client.CollectionGroup("availability_slots").
			Where("status", "==", "booked").
			Where("reminder_sent", "==", false).
			Where("start_time", ">=", windowStart.Format(time.RFC3339)).
			Where("start_time", "<=", windowEnd.Format(time.RFC3339))

		iter := query.Documents(ctx)
		reminderCount := 0

		for {
			doc, err := iter.Next()
			if err == iterator.Done {
				break
			}
			if err != nil {
				log.Printf("[ReminderJob] Error iterating slots: %v", err)
				continue
			}

			data := doc.Data()
			studentID, _ := data["booked_by_discord_id"].(string)
			startTimeStr, _ := data["start_time"].(string)
			
			if studentID == "" {
				continue
			}

			// Parse start time for a friendly message
			parsedTime, _ := time.Parse(time.RFC3339, startTimeStr)
			friendlyTime := parsedTime.Format("3:04 PM MST (Mon)")

			// Construct notification message
			msg := fmt.Sprintf("⏰ **C500 Reminder:** Your mentoring session starts soon at %s! Get your code ready and jump into the voice channel.", friendlyTime)

			// Send notification to the student via Discord
			if err := discordNotifier(studentID, msg); err != nil {
				log.Printf("[ReminderJob] Failed to send reminder to student %s: %v", studentID, err)
				continue
			}

			// Atomically flag the slot so we don't spam reminders on the next 15-min tick
			_, err = doc.Ref.Update(ctx, []firestore.Update{
				{Path: "reminder_sent", Value: true},
				{Path: "reminder_sent_at", Value: firestore.ServerTimestamp},
			})
			if err != nil {
				log.Printf("[ReminderJob] Failed to mark reminder_sent for doc %s: %v", doc.Ref.ID, err)
			} else {
				reminderCount++
			}
		}

		c.JSON(http.StatusOK, gin.H{
			"status":   "success",
			"reminders_sent": reminderCount,
			"checked_at": now.Format(time.RFC3339),
		})
	}
}
