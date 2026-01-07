<script lang="ts">
  import { hive } from '../lib/state.svelte'
  import { api } from '../lib/api'
  import Modal from './Modal.svelte'

  interface Props {
    onClose: () => void
  }

  let { onClose }: Props = $props()

  let agentId = $state(hive.selectedId ?? '')
  let title = $state('')
  let description = $state('')
  let steps = $state<string[]>([''])
  let submitting = $state(false)

  function addStep() {
    steps = [...steps, '']
  }

  function removeStep(i: number) {
    steps = steps.filter((_, idx) => idx !== i)
  }

  function updateStep(i: number, value: string) {
    steps = steps.map((s, idx) => idx === i ? value : s)
  }

  async function submit() {
    if (!agentId || !title.trim() || steps.every(s => !s.trim())) return
    
    submitting = true
    try {
      const drone = hive.agents.find(a => a.id === agentId)
      await api.createTask({
        agent_id: agentId,
        agent_name: drone?.name,
        title: title.trim(),
        description: description.trim() || undefined,
        steps: steps.filter(s => s.trim()).map(s => ({ action: s }))
      })
      await hive.refresh()
      onClose()
    } catch (e) {
      console.error('Failed to create task:', e)
    } finally {
      submitting = false
    }
  }
</script>

<Modal title="Create Task" {onClose}>
  <div class="form">
    <div class="field">
      <label for="agent">Drone</label>
      <select id="agent" bind:value={agentId}>
        <option value="">Select a drone...</option>
        {#each hive.activeDrones as drone (drone.id)}
          <option value={drone.id}>{drone.name}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="title">Title</label>
      <input id="title" type="text" bind:value={title} placeholder="Task title..." />
    </div>

    <div class="field">
      <label for="desc">Description (optional)</label>
      <textarea id="desc" bind:value={description} rows="2" placeholder="What should be done..."></textarea>
    </div>

    <div class="field">
      <label>Steps</label>
      <div class="steps">
        {#each steps as step, i (i)}
          <div class="step-row">
            <span class="step-num">{i + 1}</span>
            <input 
              type="text" 
              value={step}
              oninput={(e) => updateStep(i, e.currentTarget.value)}
              placeholder="Step action..."
            />
            {#if steps.length > 1}
              <button class="remove-btn" onclick={() => removeStep(i)}>&times;</button>
            {/if}
          </div>
        {/each}
        <button class="add-step" onclick={addStep}>+ Add step</button>
      </div>
    </div>
  </div>

  {#snippet footer()}
    <button class="btn" onclick={onClose}>Cancel</button>
    <button 
      class="btn primary" 
      onclick={submit}
      disabled={!agentId || !title.trim() || steps.every(s => !s.trim()) || submitting}
    >
      {submitting ? 'Creating...' : 'Create Task'}
    </button>
  {/snippet}
</Modal>

<style>
  .form {
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

  select {
    width: 100%;
    background: var(--bg-dark);
    border: 1px solid var(--border);
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    color: var(--text);
    transition: all 0.2s;
  }

  select:focus {
    border-color: var(--honey);
    box-shadow: 0 0 0 3px rgba(255, 179, 0, 0.15);
  }

  textarea {
    resize: vertical;
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .step-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .step-num {
    width: 26px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--honey) 0%, var(--amber) 100%);
    clip-path: var(--clip-hex);
    font-size: 11px;
    font-weight: 700;
    color: #1a1610;
    flex-shrink: 0;
  }

  .step-row input {
    flex: 1;
  }

  .remove-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--red);
    font-size: 18px;
    border-radius: var(--radius-sm);
    transition: all 0.2s;
  }

  .remove-btn:hover {
    background: var(--red-glow);
  }

  .add-step {
    padding: 12px;
    border: 1px dashed rgba(255, 179, 0, 0.3);
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    font-size: 13px;
    transition: all 0.2s;
    background: rgba(255, 179, 0, 0.02);
  }

  .add-step:hover {
    background: rgba(255, 179, 0, 0.08);
    color: var(--honey-light);
    border-color: rgba(255, 179, 0, 0.5);
  }
</style>
