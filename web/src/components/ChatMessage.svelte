<script lang="ts">
  import type { Message } from '../lib/types'

  interface Props {
    message: Message
  }

  let { message }: Props = $props()
</script>

<div class="message {message.role}">
  <div class="message-header">
    <span class="role">
      {#if message.role === 'user'}
        &#128100; You
      {:else}
        &#129302; Assistant
      {/if}
    </span>
  </div>
  <div class="content">{message.content}</div>
</div>

<style>
  .message {
    max-width: 85%;
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { 
      opacity: 0; 
      transform: translateY(12px); 
    }
    to { 
      opacity: 1; 
      transform: translateY(0); 
    }
  }

  .message.user {
    margin-left: auto;
  }

  .message-header {
    margin-bottom: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .role {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .message.user .role {
    color: var(--honey);
    flex-direction: row-reverse;
  }

  .message.assistant .role {
    color: var(--text-muted);
  }

  .content {
    padding: 16px 20px;
    border-radius: var(--radius);
    font-size: 14px;
    line-height: 1.7;
    white-space: pre-wrap;
    word-break: break-word;
    position: relative;
  }

  .message.user .content {
    background: linear-gradient(135deg, var(--honey) 0%, var(--amber) 100%);
    color: #1a1610;
    border-bottom-right-radius: 4px;
    box-shadow: 0 4px 20px rgba(255, 179, 0, 0.2);
    font-weight: 500;
  }

  .message.user .content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.1) 0%, transparent 50%);
    border-radius: inherit;
    pointer-events: none;
  }

  .message.assistant .content {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-bottom-left-radius: 4px;
    position: relative;
  }

  .message.assistant .content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 100%;
    background: linear-gradient(180deg, var(--honey) 0%, transparent 100%);
    border-radius: 3px 0 0 3px;
    opacity: 0.5;
  }
</style>
