package handlers

import (
	"context"
	"fmt"
	"net/http"

	"cloud.google.com/go/firestore"
	"github.com/gin-gonic/gin"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type EmergencyCancelRequest struct {
	MentorID string `json:"mentor_id" binding:"required"`
	SlotID   string `json:"slot_id" binding:"required"`
	Reason   string `json:"reason"`
}

// EmergencyCancelHandler handles admin requests to forcefully release a booked slot.
func EmergencyCancelHandler(client *firestore.Client, discordNotifier func(discordID string, message string) error) gin.H {
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

		// Run transaction to safely revert the slot
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

			// Perform atomic reset
			updates := []firestore.Update{
				{Path: "status", Value: "available"},
				{Path: "booked_by_discord_id", Value: nil},
				{Path: "emergency_cancelled_at", Value: firestore.ServerTimestamp},
				{Path: "cancellation_reason", Value: req.Reason},
			}

			return tx.Update(slotRef, updates)
		})

		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}

		// Notify the student via Discord DM if a booking existed
		if studentIDToNotify != "" {
			reasonText := req.Reason
			if reasonText == "" {
				reasonText = "Unforeseen schedule conflict"
			}
			dmMsg := fmt.Sprintf("⚠️ **C500 Schedule Alert:** Your upcoming mentoring session at %s has been emergency-cancelled by the admin team. Reason: *%s*. Please book a new slot via `/book-session` when you're ready!", slotStartTime, reasonText)
			
			_ = discordNotifier(studentIDToNotify, dmMsg) // Non-blocking dispatch
		}

		c.JSON(http.StatusOK, gin.H{
			"status":   "success",
			"message":  "Slot successfully emergency-cancelled and student notified.",
			"slot_id":  req.SlotID,
		})
	}
}
