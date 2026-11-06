package handlers

import (
	"context"
	"fmt"
	"cloud.google.com/go/firestore"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// BookSlot attempts to atomically transition an availability slot from "available" to "booked".
func BookSlot(ctx context.Context, client *firestore.Client, mentorID string, slotID string, studentDiscordID string) error {
	// Path: mentors/{mentorID}/availability_slots/{slotID}
	slotRef := client.Collection("mentors").Doc(mentorID).Collection("availability_slots").Doc(slotID)

	err := client.RunTransaction(ctx, func(ctx context.Context, tx *firestore.Transaction) error {
		// 1. Read the slot document within the transaction
		docSnapshot, err := tx.Get(slotRef)
		if err != nil {
			if status.Code(err) == codes.NotFound {
				return fmt.Errorf("this time slot no longer exists")
			}
			return fmt.Errorf("failed to read slot: %w", err)
		}

		// 2. Extract current status
		data := docSnapshot.Data()
		currentStatus, ok := data["status"].(string)
		if !ok || currentStatus != "available" {
			return fmt.Errorf("sorry, this slot has already been booked or is locked")
		}

		// 3. Perform the atomic write (update status and bind student)
		updates := []firestore.Update{
			{Path: "status", Value: "booked"},
			{Path: "booked_by_discord_id", Value: studentDiscordID},
			{Path: "booked_at", Value: firestore.ServerTimestamp},
		}

		return tx.Update(slotRef, updates)
	})

	if err != nil {
		return err // Return the transaction error to show in the Discord ephemeral response
	}

	return nil
}
