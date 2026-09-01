package models

type GoogleAiRequest struct {
	Prompt    string `json:"instruction"`
	Document  string `json:"document"`
	Selection string `json:"selection"`
}
