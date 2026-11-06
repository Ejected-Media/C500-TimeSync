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
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// ==========================================
// 1. STUDENT BOOKING TRANSACTION
// ==========================================

// BookSlot attempts to atomically transition an availability slot from "available" to "booked".
func BookSlot(ctx context.Context, client *firestore.Client, mentorID string, slotID string, studentDiscordID string) error {
	slotRef := client.Collection("mentors").Doc(mentorID).Collection("availability_slots").Doc(slotID)

	err := client.RunTransaction(ctx, func(ctx context.Context, tx *firestore.Transaction) error {
		docSnapshot, err := tx.Get(slotRef)
		if err != nil {
			if status.Code(err) == codes.NotFound {
				return fmt.Errorf("this time slot no longer exists")
			}
			return fmt.Errorf("failed to read slot: %w", err)
		}

		data := docSnapshot.Data()
		currentStatus, ok := data["status"].(string)
		if !ok || currentStatus != "available" {
			return fmt.Errorf("sorry, this slot has already been booked or is locked")
		}

		updates := []firestore.Update{
			{Path: "status", Value: "booked"},
			{Path: "booked_by_discord_id", Value: studentDiscordID},
			{Path: "booked_at", Value: firestore.ServerTimestamp},
			{Path: "reminder_sent", Value: false}, // Initialize reminder flag
		}

		return tx.Update(slotRef, updates)
	})

	return err
}

// ==========================================
// 2. CLOUD SCHEDULER REMINDER WORKER
// ==========================================

// CheckRemindersHandler scans for upcoming booked sessions and fires Discord notifications.
func CheckRemindersHandler(client *firestore.Client, discordNotifier func(discordID string, message string) error) gin.HandlerFunc {
	return func(c *gin.Context) {
		ctx := c.Request.Context()
		now := time.Now().UTC()
		
		windowStart := now
		windowEnd := now.Add(20 * time.Minute)

		log.Printf("[ReminderJob] Scanning for sessions starting between %s and %s", 
			windowStart.Format(time.RFC3339), windowEnd.Format(time.RFC3339))

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

			parsedTime, _ := time.Parse(time.RFC3339, startTimeStr)
			friendlyTime := parsedTime.Format("3:04 PM MST (Mon)")

			msg := fmt.Sprintf("⏰ **C500 Reminder:** Your mentoring session starts soon at %s! Get your code ready and jump into the voice channel.", friendlyTime)

			if err := discordNotifier(studentID, msg); err != nil {
				log.Printf("[ReminderJob] Failed to send reminder to student %s: %v", studentID, err)
				continue
			}

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
			"status":         "success",
			"reminders_sent": reminderCount,
			"checked_at":     now.Format(time.RFC3339),
		})
	}
}

// ==========================================
// 3. ADMIN EMERGENCY CANCELLATION
// ==========================================

type EmergencyCancelRequest struct {
	MentorID string `json:"mentor_id" binding:"required"`
	SlotID   string `json:"slot_id" binding:"required"`
	Reason   string `json:"reason"`
}

// EmergencyCancelHandler handles admin requests to forcefully release a booked slot and notify the student.
func EmergencyCancelHandler(client *firestore.Client, discordNotifier func(discordID string, message string) error) gin.HandlerFunc {
	return func(c *gin.Context) {
		var req EmergencyCancelRequest
		if err := c.ShouldBindJSON(&req); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			return
		}

		ctx := c.Request.Context()
		slotRef := client.Collection("mentors").Doc(req.MentorID).Collection("availability_slots").Doc(req.SlotID)

		var studentIDToNotify string
		var slotStartTime string

		err := client.RunTransaction(ctx, func(ctx context.Context, tx *firestore.Transaction) error {
			docSnapshot, err := tx.Get(slotRef)
			if err != nil {
				if status.Code(err) == codes.NotFound {
					return fmt.Errorf("slot not found")
				}
				return err
			}

			data := docSnapshot.Data()
			currentStatus, _ := data["status"].(string)
			if currentStatus != "booked" {
				return fmt.Errorf("slot is not currently booked (status: %s)", currentStatus)
			}

			studentIDToNotify, _ = data["booked_by_discord_id"].(string)
			slotStartTime, _ = data["start_time"].(string)

			updates := []firestore.Update{
				{Path: "status", Value: "available"},
				{Path: "booked_by_discord_id", Value: nil},
				{Path: "emergency_cancelled_at", Value: firestore.ServerTimestamp},
				{Path: "cancellation_reason", Value: req.Reason},
				{Path: "reminder_sent", Value: false},
			}

			return tx.Update(slotRef, updates)
		})

		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}

		if studentIDToNotify != "" {
			reasonText := req.Reason
			if reasonText == "" {
				reasonText = "Unforeseen schedule conflict"
			}
			dmMsg := fmt.Sprintf("⚠️ **C500 Schedule Alert:** Your upcoming mentoring session at %s has been emergency-cancelled by the admin team. Reason: *%s*. Please book a new slot via `/book-session` when you're ready!", slotStartTime, reasonText)
			
			_ = discordNotifier(studentIDToNotify, dmMsg)
		}

		c.JSON(http.StatusOK, gin.H{
			"status":  "success",
			"message": "Slot successfully emergency-cancelled and student notified.",
			"slot_id": req.SlotID,
		})
	}
}
