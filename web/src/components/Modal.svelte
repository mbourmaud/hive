<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    onClose: () => void
    children: Snippet
    footer?: Snippet
  }

  let { title, onClose, children, footer }: Props = $props()

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose()
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose()
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="overlay" onclick={handleBackdrop} onkeydown={handleKeydown} role="dialog" aria-modal="true" tabindex="-1">
  <div class="modal">
    <header class="modal-header">
      <h2>{title}</h2>
      <button class="close" onclick={onClose}>&times;</button>
    </header>
    
    <div class="modal-body">
      {@render children()}
    </div>
    
    {#if footer}
      <footer class="modal-footer">
        {@render footer()}
      </footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 13, 9, 0.85);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 480px;
    max-width: 90vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    animation: slideIn 0.25s ease-out;
    box-shadow: 
      0 0 0 1px rgba(255, 179, 0, 0.1),
      0 20px 50px rgba(0, 0, 0, 0.5),
      0 0 100px rgba(255, 179, 0, 0.05);
    position: relative;
    overflow: hidden;
  }

  .modal::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--honey), var(--amber), var(--honey));
  }

  @keyframes slideIn {
    from { opacity: 0; transform: scale(0.95) translateY(-20px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, rgba(255, 179, 0, 0.03) 0%, transparent 100%);
  }

  .modal-header h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .modal-header h2::before {
    content: '';
    width: 10px;
    height: 12px;
    background: linear-gradient(135deg, var(--honey) 0%, var(--amber) 100%);
    clip-path: var(--clip-hex);
  }

  .close {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: all 0.2s;
  }

  .close:hover {
    background: var(--red-glow);
    color: var(--red);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    background: linear-gradient(0deg, rgba(255, 179, 0, 0.02) 0%, transparent 100%);
  }
</style>
