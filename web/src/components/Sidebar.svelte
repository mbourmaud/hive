<script lang="ts">
  import { hive } from '../lib/state.svelte'
  import DroneItem from './DroneItem.svelte'

  interface Props {
    onCreateTask: () => void
    onToggleTheme: () => void
    theme: 'dark' | 'light'
  }

  let { onCreateTask, onToggleTheme, theme }: Props = $props()
</script>

<aside class="sidebar">
  <header class="sidebar-header">
    <div class="logo">
      <span class="bee">🐝</span>
      <h1>Hive</h1>
    </div>
    <div class="header-actions">
      <button class="theme-toggle" onclick={onToggleTheme} title="Toggle theme">
        {theme === 'dark' ? '☀️' : '🌙'}
      </button>
      <div class="status" class:connected={hive.connected}>
        <span class="dot"></span>
        <span>{hive.connected ? 'Live' : 'Off'}</span>
      </div>
    </div>
  </header>

  <div class="sidebar-content">
    <div class="section">
      <div class="section-header">
        <span class="section-title">Drones</span>
        <span class="count">{hive.activeDrones.length}</span>
      </div>

      {#if hive.activeDrones.length === 0}
        <div class="empty">No active drones</div>
      {:else}
        <div class="drone-list">
          {#each hive.activeDrones as drone (drone.id)}
            <DroneItem 
              {drone} 
              tasks={hive.tasksByDrone.get(drone.id) ?? []}
              selected={hive.selectedId === drone.id}
              onSelect={() => hive.selectDrone(drone.id)}
              {onCreateTask}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <footer class="sidebar-footer">
    <button class="btn primary" onclick={onCreateTask}>
      <span>+</span> New Task
    </button>
  </footer>
</aside>

<style>
  .sidebar {
    width: 260px;
    background: var(--bg-card);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    transition: background-color 0.2s;
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bee {
    font-size: 26px;
  }

  .logo h1 {
    font-size: 18px;
    font-weight: 700;
    color: var(--honey);
    letter-spacing: -0.3px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    background: var(--red-glow);
    color: var(--red);
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status.connected {
    background: var(--green-glow);
    color: var(--green);
  }

  .status .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .section-title {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .count {
    background: var(--yellow-glow);
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    color: var(--honey-dark);
  }

  .drone-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-dim);
    font-size: 12px;
    border-radius: var(--radius);
    border: 1px dashed var(--border);
  }

  .sidebar-footer {
    padding: 12px;
    border-top: 1px solid var(--border);
  }

  .sidebar-footer .btn {
    width: 100%;
  }
</style>
