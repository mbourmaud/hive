package ui

import (
	"github.com/AlecAivazis/survey/v2"
)

// PromptRequired prompts for required input with optional validation
// validator should be a function with signature func(string) error
func PromptRequired(label string, validator ...func(string) error) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
	}

	opts := []survey.AskOpt{survey.WithValidator(survey.Required)}
	if len(validator) > 0 && validator[0] != nil {
		opts = append(opts, survey.WithValidator(func(ans interface{}) error {
			if str, ok := ans.(string); ok {
				return validator[0](str)
			}
			return nil
		}))
	}

	err := survey.AskOne(prompt, &value, opts...)
	return value, err
}

// PromptDefault prompts with a default value
func PromptDefault(label, defaultValue string) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
		Default: defaultValue,
	}

	err := survey.AskOne(prompt, &value)
	if err != nil {
		return defaultValue, err
	}

	if value == "" {
		return defaultValue, nil
	}

	return value, nil
}

// PromptOptional prompts for optional input
func PromptOptional(label string) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
	}

	err := survey.AskOne(prompt, &value)
	return value, err
}

// PromptSecret prompts for sensitive input (password/token) with masking
func PromptSecret(label string) (string, error) {
	var value string
	prompt := &survey.Password{
		Message: label,
	}

	err := survey.AskOne(prompt, &value, survey.WithValidator(survey.Required))
	return value, err
}

// PromptConfirm prompts for yes/no confirmation
func PromptConfirm(label string, defaultYes bool) (bool, error) {
	var value bool
	prompt := &survey.Confirm{
		Message: label,
		Default: defaultYes,
	}

	err := survey.AskOne(prompt, &value)
	return value, err
}

// PromptSelect prompts for selection from a list
func PromptSelect(label string, options []string) (string, error) {
	var value string
	prompt := &survey.Select{
		Message: label,
		Options: options,
	}

	err := survey.AskOne(prompt, &value)
	return value, err
}

// PromptMultiSelect prompts for multiple selections from a list
func PromptMultiSelect(label string, options []string) ([]string, error) {
	var values []string
	prompt := &survey.MultiSelect{
		Message: label,
		Options: options,
	}

	err := survey.AskOne(prompt, &values)
	return values, err
}
