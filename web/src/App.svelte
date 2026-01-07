<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { hive } from './lib/state.svelte'
  import Sidebar from './components/Sidebar.svelte'
  import Header from './components/Header.svelte'
  import Chat from './components/Chat.svelte'
  import TaskModal from './components/TaskModal.svelte'
  import SolicitationModal from './components/SolicitationModal.svelte'
  import DroneContext from './components/DroneContext.svelte'
  import './app.css'

  let showTaskModal = $state(false)
  let showSolModal = $state(false)
  let theme = $state<'dark' | 'light'>('dark')

  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark'
    localStorage.setItem('hive-theme', theme)
    document.documentElement.setAttribute('data-theme', theme)
  }

  onMount(() => {
    // Load saved theme
    const saved = localStorage.getItem('hive-theme') as 'dark' | 'light' | null
    if (saved) {
      theme = saved
    }
    document.documentElement.setAttribute('data-theme', theme)
    
    // Initialize hive
    hive.init()
  })
  onDestroy(() => hive.destroy())
</script>

<div class="app" class:with-context={hive.selectedDrone}>
  <Sidebar 
    onCreateTask={() => showTaskModal = true} 
    onToggleTheme={toggleTheme}
    {theme}
  />
  
  <main class="main">
    {#if hive.selectedDrone}
      <Header 
        onCreateTask={() => showTaskModal = true}
      />
      <Chat />
    {:else}
      <div class="empty-state full">
        <div class="icon">🐝</div>
        <div class="title">Welcome to Hive</div>
        <div class="subtitle">Select a drone from the sidebar to view its conversation</div>
      </div>
    {/if}
  </main>

  {#if hive.selectedDrone}
    <DroneContext drone={hive.selectedDrone} />
  {/if}
</div>

{#if showTaskModal}
  <TaskModal onClose={() => showTaskModal = false} />
{/if}

{#if hive.pendingSolicitations.length > 0 && showSolModal}
  <SolicitationModal 
    solicitation={hive.pendingSolicitations[0]} 
    onClose={() => showSolModal = false} 
  />
{/if}

{#if hive.pendingSolicitations.length > 0 && !showSolModal}
  <button class="sol-trigger" onclick={() => showSolModal = true}>
    <span class="sol-badge">{hive.pendingSolicitations.length}</span>
    <span>Solicitation{hive.pendingSolicitations.length > 1 ? 's' : ''} pending</span>
  </button>
{/if}

<style>
  .app {
    display: grid;
    grid-template-columns: 260px 1fr;
    height: 100vh;
    overflow: hidden;
    transition: grid-template-columns 0.2s ease;
  }

  .app.with-context {
    grid-template-columns: 260px 1fr 260px;
  }

  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-dark);
    position: relative;
    overflow: hidden;
  }

  .main::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 200px;
    background: radial-gradient(ellipse at top, rgba(255, 179, 0, 0.03) 0%, transparent 70%);
    pointer-events: none;
  }

  .empty-state.full {
    flex: 1;
    justify-content: center;
    position: relative;
    z-index: 1;
  }

  .sol-trigger {
    position: fixed;
    bottom: 24px;
    right: 24px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 24px;
    background: linear-gradient(135deg, var(--red), #b62324);
    color: white;
    border-radius: 28px;
    font-weight: 600;
    box-shadow: 
      0 4px 20px rgba(239, 83, 80, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.1) inset;
    animation: solPulse 2s infinite;
    cursor: pointer;
    z-index: 100;
    transition: transform 0.2s, box-shadow 0.2s;
  }

  @keyframes solPulse {
    0%, 100% { 
      box-shadow: 
        0 4px 20px rgba(239, 83, 80, 0.4),
        0 0 0 1px rgba(255, 255, 255, 0.1) inset;
    }
    50% { 
      box-shadow: 
        0 4px 30px rgba(239, 83, 80, 0.6),
        0 0 0 1px rgba(255, 255, 255, 0.15) inset;
    }
  }

  .sol-trigger:hover {
    transform: scale(1.05) translateY(-2px);
    box-shadow: 
      0 8px 30px rgba(239, 83, 80, 0.5),
      0 0 0 1px rgba(255, 255, 255, 0.15) inset;
  }

  .sol-badge {
    width: 26px;
    height: 26px;
    background: white;
    color: var(--red);
    clip-path: var(--clip-hex);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 800;
  }
</style>
