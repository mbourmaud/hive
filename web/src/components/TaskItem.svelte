<script lang="ts">
  import type { Task } from '../lib/types'
  import { api } from '../lib/api'
  import { hive } from '../lib/state.svelte'

  interface Props {
    task: Task
  }

  let { task }: Props = $props()

  const title = $derived(task.plan?.title ?? 'Untitled')
  const totalSteps = $derived(task.plan?.steps?.length ?? 0)

  async function start() {
    await api.taskAction(task.id, 'start')
    await hive.refresh()
  }

  async function complete() {
    await api.taskAction(task.id, 'complete')
    await hive.refresh()
  }
</script>

<div class="task-item">
  <span class="status-dot {task.status}"></span>
  <span class="task-title" title={title}>{title}</span>
  <span class="progress">{task.current_step}/{totalSteps}</span>
  
  {#if task.status === 'pending' || task.status === 'assigned'}
    <button class="action start" onclick={start}>▶ Start</button>
  {:else if task.status === 'in_progress'}
    <button class="action complete" onclick={complete}>✓ Done</button>
  {:else if task.status === 'completed'}
    <span class="status-badge completed">✓</span>
  {/if}
</div>

<style>
  .task-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    transition: all 0.2s;
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.03) 0%, rgba(255, 143, 0, 0.01) 100%);
    border: 1px solid rgba(255, 179, 0, 0.08);
  }

  .task-item:hover {
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.06) 0%, rgba(255, 143, 0, 0.03) 100%);
    border-color: rgba(255, 179, 0, 0.15);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.pending, .status-dot.assigned {
    background: var(--text-dim);
    box-shadow: 0 0 6px rgba(150, 140, 120, 0.3);
  }

  .status-dot.in_progress {
    background: var(--honey);
    box-shadow: 0 0 8px rgba(255, 179, 0, 0.5);
    animation: pulse 2s infinite;
  }

  .status-dot.completed {
    background: var(--green);
    box-shadow: 0 0 6px rgba(139, 195, 74, 0.4);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.7; transform: scale(1.1); }
  }

  .task-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    font-weight: 500;
  }

  .progress {
    font-size: 10px;
    color: var(--honey);
    font-family: 'JetBrains Mono', monospace;
    background: rgba(255, 179, 0, 0.12);
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid rgba(255, 179, 0, 0.15);
  }

  .action {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    transition: all 0.2s;
    cursor: pointer;
  }

  .action.start {
    background: linear-gradient(135deg, rgba(139, 195, 74, 0.15) 0%, rgba(139, 195, 74, 0.08) 100%);
    color: var(--green);
    border: 1px solid rgba(139, 195, 74, 0.25);
  }

  .action.start:hover {
    background: linear-gradient(135deg, rgba(139, 195, 74, 0.25) 0%, rgba(139, 195, 74, 0.15) 100%);
    border-color: rgba(139, 195, 74, 0.4);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(139, 195, 74, 0.2);
  }

  .action.complete {
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.15) 0%, rgba(255, 143, 0, 0.08) 100%);
    color: var(--honey);
    border: 1px solid rgba(255, 179, 0, 0.25);
  }

  .action.complete:hover {
    background: linear-gradient(135deg, rgba(255, 179, 0, 0.25) 0%, rgba(255, 143, 0, 0.15) 100%);
    border-color: rgba(255, 179, 0, 0.4);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(255, 179, 0, 0.2);
  }

  .status-badge.completed {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(139, 195, 74, 0.15);
    color: var(--green);
    border-radius: 50%;
    font-size: 11px;
  }
</style>
