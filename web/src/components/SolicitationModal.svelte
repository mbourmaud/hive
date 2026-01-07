<script lang="ts">
  import type { Solicitation } from '../lib/types'
  import { api } from '../lib/api'
  import { hive } from '../lib/state.svelte'
  import Modal from './Modal.svelte'

  interface Props {
    solicitation: Solicitation
    onClose: () => void
  }

  let { solicitation, onClose }: Props = $props()

  let response = $state('')
  let submitting = $state(false)

  function selectOption(opt: string) {
    response = opt
  }

  async function respond() {
    if (!response.trim()) return
    submitting = true
    try {
      await api.respondSolicitation(solicitation.id, response)
      await hive.refresh()
      onClose()
    } finally {
      submitting = false
    }
  }

  async function dismiss() {
    if (!confirm('Dismiss this solicitation?')) return
    await api.dismissSolicitation(solicitation.id)
    await hive.refresh()
    onClose()
  }
</script>

<Modal title="{solicitation.type} from {solicitation.agent_name}" {onClose}>
  <div class="content">
    <div class="field">
      <label>Message</label>
      <div class="message-box">{solicitation.message}</div>
    </div>

    {#if solicitation.options?.length}
      <div class="field">
        <label>Options</label>
        <div class="options">
          {#each solicitation.options as opt (opt)}
            <button 
              class="option" 
              class:selected={response === opt}
              onclick={() => selectOption(opt)}
            >
              {opt}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="field">
      <label for="response">Your Response</label>
      <textarea 
        id="response" 
        bind:value={response} 
        rows="3" 
        placeholder="Type your response..."
      ></textarea>
    </div>
  </div>

  {#snippet footer()}
    <button class="btn danger" onclick={dismiss}>Dismiss</button>
    <button 
      class="btn primary" 
      onclick={respond}
      disabled={!response.trim() || submitting}
    >
      {submitting ? 'Sending...' : 'Respond'}
    </button>
  {/snippet}
</Modal>

<style>
  .content {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field label {
    font-size: 11px;
    font-weight: 700;
    color: var(--honey);
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .message-box {
    padding: 16px;
    background: var(--bg-dark);
    border-radius: var(--radius);
    border: 1px solid var(--border);
    font-size: 13px;
    line-height: 1.7;
    max-height: 150px;
    overflow-y: auto;
    white-space: pre-wrap;
    position: relative;
  }

  .message-box::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 100%;
    background: linear-gradient(180deg, var(--honey) 0%, transparent 100%);
    border-radius: 3px 0 0 3px;
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .option {
    padding: 14px 18px;
    background: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-align: left;
    color: var(--text);
    font-size: 13px;
    transition: all 0.2s;
    position: relative;
    padding-left: 36px;
  }

  .option::before {
    content: '';
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    width: 10px;
    height: 12px;
    background: var(--border);
    clip-path: var(--clip-hex);
    transition: background 0.2s;
  }

  .option:hover {
    border-color: var(--honey);
    background: rgba(255, 179, 0, 0.05);
  }

  .option:hover::before {
    background: var(--honey);
  }

  .option.selected {
    background: var(--bg-selected);
    border-color: var(--honey);
    box-shadow: 0 0 20px rgba(255, 179, 0, 0.1);
  }

  .option.selected::before {
    background: var(--honey);
  }

  textarea {
    resize: vertical;
  }
</style>
