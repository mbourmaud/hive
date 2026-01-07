<script lang="ts">
  import { hive } from '../lib/state.svelte'

  interface Props {
    onCreateTask: () => void
  }

  let { onCreateTask }: Props = $props()

  const drone = $derived(hive.selectedDrone)

  async function kill() {
    if (!drone || !confirm(`Stop drone "${drone.name}"?`)) return
    await hive.killDrone()
  }

  async function destroy() {
    if (!drone || !confirm(`Destroy drone "${drone.name}"? This removes its worktree.`)) return
    await hive.destroyDrone()
  }
</script>

{#if drone}
  <header class="header">
    <div class="drone-info">
      <h2>{drone.name}</h2>
      <span class="badge {drone.status}">{drone.status}</span>
      {#if drone.specialty}
        <span class="badge specialty">{drone.specialty}</span>
      {/if}
    </div>
    
    <div class="actions">
      <button class="btn sm" onclick={onCreateTask}>+ Task</button>
      <button class="btn sm" onclick={kill}>Stop</button>
      <button class="btn sm danger" onclick={destroy}>Destroy</button>
    </div>
  </header>
{/if}

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    position: relative;
  }

  .header::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(90deg, var(--honey) 0%, transparent 20%, transparent 80%, var(--honey) 100%);
    opacity: 0.3;
  }

  .drone-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  h2::before {
    content: '';
    width: 12px;
    height: 14px;
    background: linear-gradient(135deg, var(--honey) 0%, var(--amber) 100%);
    clip-path: var(--clip-hex);
    flex-shrink: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
  }
</style>
