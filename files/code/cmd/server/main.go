package main

import (
	"context"
	"log"
	"net/http"
	"os"

	"cloud.google.com/go/firestore"
	"github.com/gin-gonic/gin"
	
	// Import your internal handlers package (adjust import path to your module name)
	// "github.com/Ejected-Media/C500-TimeSync/internal/handlers"
)

func main() {
	ctx := context.Background()
	projectID := os.Getenv("GCP_PROJECT_ID")
	if projectID == "" {
		projectID = "c500-prod" // Default fallback or read from env
	}

	// 1. Initialize Firebase Firestore Client
	client, err := firestore.NewClient(ctx, projectID)
	if err != nil {
		log.Fatalf("Failed to create Firestore client: %v", err)
	}
	defer client.Close()

	// 2. Initialize Gin Router
	r := gin.Default()

	// Mock or real Discord notifier closure (integrates with your Discord bot API/webhook)
	discordNotifier := func(discordID string, message string) error {
		log.Printf("[Discord Dispatch] Sending DM to user %s: %s", discordID, message)
		// Implement actual REST call to Discord API to send DM here
		return nil
	}

	// 3. Health Check Route
	r.GET("/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"status": "healthy",
			"system": "C500-TimeSync Go Backend",
		})
	})

	// 4. API Route Group
	api := r.Group("/api/v1")
	{
		// Student Booking Endpoint
		api.POST("/slots/book", func(c *gin.Context) {
			// Bind request payload: mentor_id, slot_id, student_discord_id
			var req struct {
				MentorID         string `json:"mentor_id" binding:"required"`
				SlotID           string `json:"slot_id" binding:"required"`
				StudentDiscordID string `json:"student_discord_id" binding:"required"`
			}
			if err := c.ShouldBindJSON(&req); err != nil {
				c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
				return
			}

			err := handlers.BookSlot(c.Request.Context(), client, req.MentorID, req.SlotID, req.StudentDiscordID)
			if err != nil {
				c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
				return
			}

			c.JSON(http.StatusOK, gin.H{"message": "Slot booked successfully!"})
		})

		// Cloud Scheduler Cron Endpoint (15-Minute Smart Reminders)
		api.POST("/tasks/check-reminders", handlers.CheckRemindersHandler(client, discordNotifier))

		// Admin Emergency Cancel Endpoint
		api.POST("/admin/emergency-cancel", handlers.EmergencyCancelHandler(client, discordNotifier))
	}

	// 5. Start Server
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	log.Printf("[C500-Server] Starting time-sync microservice on port %s...", port)
	if err := r.Run(":" + port); err != nil {
		log.Fatalf("Server shutdown: %v", err)
	}
}
