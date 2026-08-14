<script lang="ts">
  import { FileText, Bookmark, Layers, Volume2, Edit3 } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  export type StudioTab =
    'transcript' | 'chapters' | 'flashcards' | 'audio' | 'notes';

  let {
    activeTab = $bindable<StudioTab>('transcript'),
  }: {
    activeTab: StudioTab;
  } = $props();

  function handleTab(tab: StudioTab) {
    triggerHaptic('light');
    activeTab = tab;
  }
</script>

<div class="studio-nav-tabs" role="tablist" aria-label="Lecture studio tools">
  <button
    type="button"
    role="tab"
    class="tab-btn"
    class:active={activeTab === 'transcript'}
    onclick={() => handleTab('transcript')}
  >
    <FileText size={13} />
    <span>Transcript</span>
  </button>
  <button
    type="button"
    role="tab"
    class="tab-btn"
    class:active={activeTab === 'chapters'}
    onclick={() => handleTab('chapters')}
  >
    <Bookmark size={13} />
    <span>Chapters</span>
  </button>
  <button
    type="button"
    role="tab"
    class="tab-btn"
    class:active={activeTab === 'flashcards'}
    onclick={() => handleTab('flashcards')}
  >
    <Layers size={13} />
    <span>Flashcards</span>
  </button>
  <button
    type="button"
    role="tab"
    class="tab-btn"
    class:active={activeTab === 'audio'}
    onclick={() => handleTab('audio')}
  >
    <Volume2 size={13} />
    <span>Audio</span>
  </button>
  <button
    type="button"
    role="tab"
    class="tab-btn"
    class:active={activeTab === 'notes'}
    onclick={() => handleTab('notes')}
  >
    <Edit3 size={13} />
    <span>Notes</span>
  </button>
</div>

<style>
  .studio-nav-tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--color-cork-border);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .studio-nav-tabs::-webkit-scrollbar {
    display: none;
  }
  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-driftwood);
    cursor: pointer;
    white-space: nowrap;
    min-height: 40px;
  }
  .tab-btn:hover {
    color: var(--color-warm-cream);
  }
  .tab-btn.active {
    color: var(--color-warm-cream);
    border-bottom: 2px solid var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.06);
  }
</style>
