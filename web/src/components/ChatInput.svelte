<script lang="ts">
  import { hive } from '../lib/state.svelte'

  let input = $state('')
  let sending = $state(false)

  async function send() {
    if (!input.trim() || sending) return
    sending = true
    try {
      await hive.sendMessage(input)
      input = ''
    } finally {
      sending = false
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      send()
    }
  }
</script>

<div class="input-area">
  <input 
    type="text" 
    bind:value={input}
    onkeydown={handleKeydown}
    placeholder="Send a message to the drone..."
    disabled={sending}
  />
  <button 
    class="btn primary send" 
    onclick={send}
    disabled={!input.trim() || sending}
  >
    {#if sending}
      Sending...
    {:else}
      Send
    {/if}
  </button>
</div>

<style>
  .input-area {
    display: flex;
    gap: 12px;
    padding: 20px 24px;
    background: var(--bg-card);
    border-top: 1px solid var(--border);
    position: relative;
  }

  .input-area::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent 0%, var(--honey) 50%, transparent 100%);
    opacity: 0.2;
  }

  input {
    flex: 1;
    padding: 14px 18px;
    font-size: 14px;
    border-radius: var(--radius);
    background: var(--bg-dark);
    border: 1px solid var(--border);
    transition: all 0.2s;
  }

  input:focus {
    border-color: var(--honey);
    box-shadow: 0 0 0 3px rgba(255, 179, 0, 0.15);
  }

  input::placeholder {
    color: var(--text-dim);
  }

  .send {
    padding: 14px 28px;
    font-size: 14px;
    position: relative;
    overflow: hidden;
  }

  .send::after {
    content: '';
    position: absolute;
    width: 12px;
    height: 14px;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(26, 22, 16, 0.3);
    clip-path: var(--clip-hex);
    opacity: 0;
    transition: opacity 0.2s;
  }

  .send:hover::after {
    opacity: 1;
  }

  .send:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    background: var(--border);
    border-color: var(--border);
  }
</style>
