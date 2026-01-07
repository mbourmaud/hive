<script lang="ts">
  import type { Agent, Task } from '../lib/types'
  import TaskItem from './TaskItem.svelte'

  interface Props {
    drone: Agent
    tasks: Task[]
    selected: boolean
    onSelect: () => void
    onCreateTask: () => void
  }

  let { drone, tasks, selected, onSelect, onCreateTask }: Props = $props()
  let expanded = $state(false)

  function toggleExpand(e: Event) {
    e.stopPropagation()
    expanded = !expanded
  }

  $effect(() => {
    if (selected && tasks.length > 0) expanded = true
  })
</script>

<div class="drone-group" class:expanded class:selected>
  <button class="drone-item" onclick={onSelect}>
    <span class="status-dot {drone.status}"></span>
    <div class="drone-info">
      <div class="drone-name">
        {drone.name}
        {#if drone.specialty}
          <span class="badge specialty">{drone.specialty}</span>
        {/if}
      </div>
      <div class="drone-meta">:{drone.port}</div>
    </div>
    {#if tasks.length > 0}
      <span class="expand-btn" onclick={toggleExpand} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && toggleExpand(e)}>
        <span class="chevron">&#9658;</span>
        <span class="task-count">{tasks.length}</span>
      </span>
    {/if}
  </button>

  {#if expanded && tasks.length > 0}
    <div class="tasks">
      {#each tasks as task (task.id)}
        <TaskItem {task} />
      {/each}
      <button class="add-task-btn" onclick={onCreateTask}>+ Add task</button>
    </div>
  {/if}
</div>

<style>
  .drone-group {
    background: var(--bg-dark);
    border-radius: var(--radius);
    overflow: hidden;
    transition: all 0.2s ease;
    border: 1px solid transparent;
    position: relative;
  }

  .drone-group::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 3px;
    height: 100%;
    background: linear-gradient(180deg, var(--honey) 0%, var(--amber-dark) 100%);
    opacity: 0;
    transition: opacity 0.2s;
  }

  .drone-group:hover::before {
    opacity: 0.5;
  }

  .drone-group.selected {
    background: var(--bg-selected);
    border-color: rgba(255, 179, 0, 0.3);
    box-shadow: 0 0 20px rgba(255, 179, 0, 0.1);
  }

  .drone-group.selected::before {
    opacity: 1;
  }

  .drone-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 14px 12px;
    text-align: left;
    color: var(--text);
    transition: background 0.2s;
  }

  .drone-item:hover {
    background: var(--bg-hover);
  }

  .drone-info {
    flex: 1;
    min-width: 0;
  }

  .drone-name {
    font-weight: 600;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .drone-meta {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 3px;
    font-family: 'JetBrains Mono', monospace;
  }

  .expand-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.1) 0%, rgba(255, 143, 0, 0.05) 100%);
    border: 1px solid rgba(255, 179, 0, 0.15);
    border-radius: var(--radius-sm);
    color: var(--honey-light);
    font-size: 11px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .expand-btn:hover {
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.2) 0%, rgba(255, 143, 0, 0.1) 100%);
    border-color: rgba(255, 179, 0, 0.3);
    color: var(--honey);
  }

  .chevron {
    font-size: 8px;
    transition: transform 0.2s ease;
    color: var(--honey);
  }

  .expanded .chevron {
    transform: rotate(90deg);
  }

  .tasks {
    padding: 0 12px 12px 32px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-top: 1px dashed var(--border);
    margin-top: 4px;
    padding-top: 10px;
  }

  .add-task-btn {
    padding: 8px;
    color: var(--text-dim);
    font-size: 11px;
    border-radius: var(--radius-sm);
    transition: all 0.2s;
    text-align: left;
    border: 1px dashed transparent;
  }

  .add-task-btn:hover {
    background: rgba(255, 179, 0, 0.05);
    color: var(--honey-light);
    border-color: rgba(255, 179, 0, 0.2);
  }
</style>
