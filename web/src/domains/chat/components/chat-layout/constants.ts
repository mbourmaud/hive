export const INITIAL_RENDER_COUNT = 20;
export const SCROLL_THRESHOLD = 100; // px from bottom to consider "at bottom"

export const SUGGESTION_PROMPTS = [
  {
    label: "Fix a bug",
    hint: "Describe the issue and I'll track it down",
    prompt: "I have a bug where ",
  },
  {
    label: "Add a feature",
    hint: "Tell me what to build",
    prompt: "I want to add a feature that ",
  },
  {
    label: "Explain this code",
    hint: "Paste or point me to the code",
    prompt: "Can you explain the code in ",
  },
  { label: "Write tests", hint: "I'll generate tests for your code", prompt: "Write tests for " },
] as const;
