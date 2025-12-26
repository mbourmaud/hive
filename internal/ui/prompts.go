package ui

import (
	"os"

	"github.com/AlecAivazis/survey/v2"
	"github.com/AlecAivazis/survey/v2/terminal"
)

// defaultStdio returns the default terminal stdio (os.Stdin, os.Stdout, os.Stderr)
func defaultStdio() terminal.Stdio {
	return terminal.Stdio{In: os.Stdin, Out: os.Stdout, Err: os.Stderr}
}

// PromptRequired prompts for required input with optional validation
// validator should be a function with signature func(string) error
func PromptRequired(label string, validator ...func(string) error) (string, error) {
	return PromptRequiredWithStdio(label, defaultStdio(), validator...)
}

// PromptDefault prompts with a default value
func PromptDefault(label, defaultValue string) (string, error) {
	return PromptDefaultWithStdio(label, defaultValue, defaultStdio())
}

// PromptOptional prompts for optional input
func PromptOptional(label string) (string, error) {
	return PromptOptionalWithStdio(label, defaultStdio())
}

// PromptSecret prompts for sensitive input (password/token) with masking
func PromptSecret(label string) (string, error) {
	return PromptSecretWithStdio(label, defaultStdio())
}

// PromptConfirm prompts for yes/no confirmation
func PromptConfirm(label string, defaultYes bool) (bool, error) {
	return PromptConfirmWithStdio(label, defaultYes, defaultStdio())
}

// PromptSelect prompts for selection from a list
func PromptSelect(label string, options []string) (string, error) {
	return PromptSelectWithStdio(label, options, defaultStdio())
}

// PromptMultiSelect prompts for multiple selections from a list
func PromptMultiSelect(label string, options []string) ([]string, error) {
	return PromptMultiSelectWithStdio(label, options, defaultStdio())
}

// =============================================================================
// WithStdio variants for testing with virtual terminals
// =============================================================================

// PromptRequiredWithStdio is like PromptRequired but with custom stdio for testing
func PromptRequiredWithStdio(label string, stdio terminal.Stdio, validator ...func(string) error) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
	}

	opts := []survey.AskOpt{
		survey.WithValidator(survey.Required),
		survey.WithStdio(stdio.In, stdio.Out, stdio.Err),
	}
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

// PromptDefaultWithStdio is like PromptDefault but with custom stdio for testing
func PromptDefaultWithStdio(label, defaultValue string, stdio terminal.Stdio) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
		Default: defaultValue,
	}

	err := survey.AskOne(prompt, &value, survey.WithStdio(stdio.In, stdio.Out, stdio.Err))
	if err != nil {
		return defaultValue, err
	}

	if value == "" {
		return defaultValue, nil
	}

	return value, nil
}

// PromptOptionalWithStdio is like PromptOptional but with custom stdio for testing
func PromptOptionalWithStdio(label string, stdio terminal.Stdio) (string, error) {
	var value string
	prompt := &survey.Input{
		Message: label,
	}

	err := survey.AskOne(prompt, &value, survey.WithStdio(stdio.In, stdio.Out, stdio.Err))
	return value, err
}

// PromptSecretWithStdio is like PromptSecret but with custom stdio for testing
func PromptSecretWithStdio(label string, stdio terminal.Stdio) (string, error) {
	var value string
	prompt := &survey.Password{
		Message: label,
	}

	err := survey.AskOne(prompt, &value,
		survey.WithValidator(survey.Required),
		survey.WithStdio(stdio.In, stdio.Out, stdio.Err),
	)
	return value, err
}

// PromptConfirmWithStdio is like PromptConfirm but with custom stdio for testing
func PromptConfirmWithStdio(label string, defaultYes bool, stdio terminal.Stdio) (bool, error) {
	var value bool
	prompt := &survey.Confirm{
		Message: label,
		Default: defaultYes,
	}

	err := survey.AskOne(prompt, &value, survey.WithStdio(stdio.In, stdio.Out, stdio.Err))
	return value, err
}

// PromptSelectWithStdio is like PromptSelect but with custom stdio for testing
func PromptSelectWithStdio(label string, options []string, stdio terminal.Stdio) (string, error) {
	var value string
	prompt := &survey.Select{
		Message: label,
		Options: options,
	}

	err := survey.AskOne(prompt, &value, survey.WithStdio(stdio.In, stdio.Out, stdio.Err))
	return value, err
}

// PromptMultiSelectWithStdio is like PromptMultiSelect but with custom stdio for testing
func PromptMultiSelectWithStdio(label string, options []string, stdio terminal.Stdio) ([]string, error) {
	var values []string
	prompt := &survey.MultiSelect{
		Message: label,
		Options: options,
	}

	err := survey.AskOne(prompt, &values, survey.WithStdio(stdio.In, stdio.Out, stdio.Err))
	return values, err
}
