<script lang="ts">
  import { hive } from '../lib/state.svelte'
  import ChatMessage from './ChatMessage.svelte'
  import ChatInput from './ChatInput.svelte'

  let chatArea: HTMLElement

  $effect(() => {
    if (hive.conversation.length && chatArea) {
      chatArea.scrollTop = chatArea.scrollHeight
    }
  })
</script>

<div class="chat">
  <div class="messages" bind:this={chatArea}>
    {#if hive.conversation.length === 0}
      <div class="empty-state">
        <div class="icon">&#128172;</div>
        <div class="title">No messages yet</div>
        <div class="subtitle">Send a message to start the conversation</div>
      </div>
    {:else}
      {#each hive.conversation as message, i (i)}
        <ChatMessage {message} />
      {/each}
    {/if}
  </div>
  
  <ChatInput />
</div>

<style>
  .chat {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    background: var(--bg-chat);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: radial-gradient(ellipse at center, rgba(255, 179, 0, 0.03) 0%, transparent 70%);
  }

  .empty-state .icon {
    font-size: 64px;
    filter: drop-shadow(0 0 30px rgba(255, 179, 0, 0.3)) grayscale(0.3);
    opacity: 0.6;
  }
</style>
