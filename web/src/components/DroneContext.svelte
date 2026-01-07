<script lang="ts">
  import { hive } from '../lib/state.svelte'
  import type { Agent } from '../lib/types'

  interface Props {
    drone: Agent
  }

  let { drone }: Props = $props()
  
  let expanded = $state<Record<string, boolean>>({
    info: true,
    todos: true,
    files: false,
    context: false
  })

  function toggle(section: string) {
    expanded[section] = !expanded[section]
  }

  // Mock data for now - will be fetched from API
  const todos = $derived([
    { id: '1', content: 'Implement login flow', status: 'completed' },
    { id: '2', content: 'Add validation', status: 'in_progress' },
    { id: '3', content: 'Write tests', status: 'pending' },
  ])

  const modifiedFiles = $derived([
    { path: 'src/auth/login.ts', status: 'modified' },
    { path: 'src/components/Form.tsx', status: 'added' },
  ])

  const contextItems = $derived([
    { type: 'mcp', name: 'hive', status: 'connected' },
    { type: 'lsp', name: 'typescript', status: 'running' },
  ])

  const tasks = hive.tasksByDrone.get(drone.id) ?? []
</script>

<aside class="context-sidebar">
  <header class="context-header">
    <span class="icon">📋</span>
    <span class="title">Drone Context</span>
  </header>

  <div class="sections">
    <section class="section">
      <button class="section-header" onclick={() => toggle('info')}>
        <span class="chevron" class:open={expanded.info}>▶</span>
        <span class="section-title">Info</span>
      </button>
      {#if expanded.info}
        <div class="section-content">
          <div class="info-grid">
            <div class="info-item">
              <span class="label">Branch</span>
              <span class="value branch">
                <span class="branch-icon">⎇</span>
                {drone.branch}
              </span>
            </div>
            <div class="info-item">
              <span class="label">Port</span>
              <span class="value mono">:{drone.port}</span>
            </div>
            <div class="info-item">
              <span class="label">Status</span>
              <span class="value status {drone.status}">{drone.status}</span>
            </div>
            {#if drone.specialty}
              <div class="info-item">
                <span class="label">Specialty</span>
                <span class="value tag">{drone.specialty}</span>
              </div>
            {/if}
          </div>
          <div class="info-item full">
            <span class="label">Worktree</span>
            <span class="value path" title={drone.worktree_path}>
              {drone.worktree_path.split('/').slice(-2).join('/')}
            </span>
          </div>
        </div>
      {/if}
    </section>

    <section class="section">
      <button class="section-header" onclick={() => toggle('todos')}>
        <span class="chevron" class:open={expanded.todos}>▶</span>
        <span class="section-title">Tasks</span>
        <span class="badge">{tasks.length}</span>
      </button>
      {#if expanded.todos}
        <div class="section-content">
          {#if tasks.length === 0}
            <div class="empty">No tasks assigned</div>
          {:else}
            <ul class="todo-list">
              {#each tasks as task (task.id)}
                <li class="todo-item {task.status}">
                  <span class="todo-status">
                    {#if task.status === 'completed'}✓
                    {:else if task.status === 'in_progress'}◐
                    {:else}○{/if}
                  </span>
                  <span class="todo-text">{task.plan?.title ?? 'Untitled'}</span>
                  <span class="todo-progress">{task.current_step}/{task.plan?.steps?.length ?? 0}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </section>

    <section class="section">
      <button class="section-header" onclick={() => toggle('files')}>
        <span class="chevron" class:open={expanded.files}>▶</span>
        <span class="section-title">Modified Files</span>
        <span class="badge">{modifiedFiles.length}</span>
      </button>
      {#if expanded.files}
        <div class="section-content">
          {#if modifiedFiles.length === 0}
            <div class="empty">No modified files</div>
          {:else}
            <ul class="file-list">
              {#each modifiedFiles as file (file.path)}
                <li class="file-item">
                  <span class="file-status {file.status}">
                    {#if file.status === 'added'}A
                    {:else if file.status === 'modified'}M
                    {:else if file.status === 'deleted'}D
                    {:else}?{/if}
                  </span>
                  <span class="file-path">{file.path}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </section>

    <section class="section">
      <button class="section-header" onclick={() => toggle('context')}>
        <span class="chevron" class:open={expanded.context}>▶</span>
        <span class="section-title">Context</span>
        <span class="badge">{contextItems.length}</span>
      </button>
      {#if expanded.context}
        <div class="section-content">
          {#if contextItems.length === 0}
            <div class="empty">No context items</div>
          {:else}
            <ul class="context-list">
              {#each contextItems as item (item.name)}
                <li class="context-item">
                  <span class="context-type {item.type}">{item.type.toUpperCase()}</span>
                  <span class="context-name">{item.name}</span>
                  <span class="context-status {item.status}">●</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </section>
  </div>
</aside>

<style>
  .context-sidebar {
    width: 260px;
    background: var(--bg-card);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }

  .context-header {
    padding: 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 10px;
    background: linear-gradient(180deg, rgba(255, 179, 0, 0.03) 0%, transparent 100%);
  }

  .context-header .icon {
    font-size: 18px;
  }

  .context-header .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .sections {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .section {
    margin-bottom: 4px;
  }

  .section-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    transition: background 0.15s;
    cursor: pointer;
  }

  .section-header:hover {
    background: var(--bg-hover);
  }

  .chevron {
    font-size: 8px;
    color: var(--text-dim);
    transition: transform 0.2s;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .section-title {
    flex: 1;
  }

  .badge {
    font-size: 10px;
    padding: 2px 7px;
    background: rgba(255, 179, 0, 0.1);
    color: var(--honey);
    border-radius: 10px;
    font-weight: 600;
  }

  .section-content {
    padding: 8px 12px 12px 28px;
  }

  .empty {
    font-size: 11px;
    color: var(--text-dim);
    font-style: italic;
  }

  /* Info Grid */
  .info-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .info-item.full {
    grid-column: 1 / -1;
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
  }

  .info-item .label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-dim);
  }

  .info-item .value {
    font-size: 12px;
    color: var(--text);
  }

  .value.mono {
    font-family: 'JetBrains Mono', monospace;
  }

  .value.branch {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--honey);
    font-size: 11px;
  }

  .branch-icon {
    opacity: 0.7;
  }

  .value.status {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .value.status.ready {
    background: rgba(139, 195, 74, 0.15);
    color: var(--green);
  }

  .value.status.busy {
    background: rgba(255, 179, 0, 0.15);
    color: var(--honey);
  }

  .value.tag {
    padding: 2px 6px;
    background: rgba(255, 179, 0, 0.1);
    color: var(--honey-light);
    border-radius: 4px;
    font-size: 10px;
  }

  .value.path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-muted);
    word-break: break-all;
  }

  /* Todo List */
  .todo-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .todo-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    padding: 6px 8px;
    background: rgba(255, 179, 0, 0.02);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
  }

  .todo-item:hover {
    border-color: var(--border);
  }

  .todo-status {
    width: 14px;
    text-align: center;
    font-size: 10px;
  }

  .todo-item.completed .todo-status {
    color: var(--green);
  }

  .todo-item.in_progress .todo-status {
    color: var(--honey);
  }

  .todo-item.pending .todo-status,
  .todo-item.assigned .todo-status {
    color: var(--text-dim);
  }

  .todo-text {
    flex: 1;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .todo-item.completed .todo-text {
    text-decoration: line-through;
    opacity: 0.6;
  }

  .todo-progress {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    color: var(--honey);
    background: rgba(255, 179, 0, 0.1);
    padding: 1px 5px;
    border-radius: 4px;
  }

  /* File List */
  .file-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    padding: 4px 0;
  }

  .file-status {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    border-radius: 3px;
  }

  .file-status.added {
    background: rgba(139, 195, 74, 0.2);
    color: var(--green);
  }

  .file-status.modified {
    background: rgba(255, 179, 0, 0.2);
    color: var(--honey);
  }

  .file-status.deleted {
    background: rgba(239, 83, 80, 0.2);
    color: var(--red);
  }

  .file-path {
    flex: 1;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Context List */
  .context-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .context-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    padding: 6px 8px;
    background: var(--bg-dark);
    border-radius: var(--radius-sm);
  }

  .context-type {
    font-size: 8px;
    font-weight: 700;
    padding: 2px 5px;
    border-radius: 3px;
    letter-spacing: 0.3px;
  }

  .context-type.mcp {
    background: rgba(156, 39, 176, 0.2);
    color: #ce93d8;
  }

  .context-type.lsp {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .context-name {
    flex: 1;
    color: var(--text);
  }

  .context-status {
    font-size: 8px;
  }

  .context-status.connected,
  .context-status.running {
    color: var(--green);
  }

  .context-status.disconnected,
  .context-status.stopped {
    color: var(--red);
  }
</style>
