func HandleEmergencyCancelCommand(s *discordgo.Session, i *discordgo.InteractionCreate, apiClient *http.Client, backendURL string) {
	// 1. Verify user has the C500 Admin / Mentor Role
	if !hasAdminRole(i.Member) {
		s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
			Type: discordgo.InteractionResponseChannelMessageWithSource,
			Data: &discordgo.InteractionResponseData{
				Content: "⛔ You do not have permission to execute emergency cancellations.",
				Flags:   discordgo.MessageFlagsEphemeral,
			},
		})
		return
	}

	// 2. Extract options (mentor_id, slot_id, reason)
	// ... (parse interaction options)

	// 3. Call backend Go API endpoint
	// ... (POST to backendURL + "/admin/emergency-cancel")

	// 4. Respond ephemerally to the admin
	s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
		Type: discordgo.InteractionResponseChannelMessageWithSource,
		Data: &discordgo.InteractionResponseData{
			Content: "🚨 Emergency cancellation executed. Slot freed and student DM dispatched.",
			Flags:   discordgo.MessageFlagsEphemeral,
		},
	})
}
