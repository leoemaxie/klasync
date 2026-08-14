<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import TranscriptViewer from './TranscriptViewer.svelte';
  import ChapterBreakdown from './ChapterBreakdown.svelte';
  import FlashcardDeck from './FlashcardDeck.svelte';
  import AudioPlayerPanel from './AudioPlayerPanel.svelte';
  import LectureStudyNotes from './LectureStudyNotes.svelte';
  import {
    FileText,
    Bookmark,
    Layers,
    Volume2,
    Edit3,
    ArrowLeft,
    CheckCircle2,
    Copy,
    Share2,
    Calendar,
    Check,
  } from '@lucide/svelte';

  let {
    claim,
    onBack,
  }: {
    claim: ClaimRecord;
    onBack?: () => void;
  } = $props();

  let activeTab = $state<'transcript' | 'chapters' | 'flashcards' | 'audio' | 'notes'>('transcript');
  let isCopied = $state(false);

  const transcriptText = $derived.by(() => {
    if (typeof localStorage === 'undefined') return '';
    try {
      const storedCaptions =
        localStorage.getItem(`klasync-captions-${claim.id}`) ||
        localStorage.getItem('klasync-captions');
      if (storedCaptions) {
        const parsed = JSON.parse(storedCaptions);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed
            .map((c: any) =>
              typeof c === 'string' ? c : c.text || c.content || ''
            )
            .filter(Boolean)
            .join('\n');
        }
      }
    } catch {}
    return [
      "[00:01] Welcome everyone to today's lecture session.",
      '[00:15] Today we are discussing key principles, system architecture, and accessibility.',
      '[01:30] Make sure to review the core formulas and chapter breakdowns provided in your archive.',
      '[03:45] Any questions asked during class have been synced to your student Q&A record.',
    ].join('\n');
  });

  function handleCopyShareLink() {
    const url = `${window.location.origin}/#archive?lecture=${encodeURIComponent(claim.id)}`;
    navigator.clipboard.writeText(url).then(() => {
      isCopied = true;
      setTimeout(() => (isCopied = false), 2000);
    });
  }
</script>

<div class="lecture-studio-pane">
  <!-- Top Command & Navigation Bar -->
  <div class="studio-header">
    <div class="header-left">
      {#if onBack}
        <button type="button" class="back-btn outline" onclick={onBack} title="Back to course explorer">
          <ArrowLeft size={14} />
          <span>Explorer</span>
        </button>
      {/if}

      <div class="meta-tags-row">
        <span class="course-code-badge">{claim.course_code}</span>
        {#if claim.course_title && claim.course_title !== claim.course_code}
          <span class="course-title-sub">{claim.course_title}</span>
        {/if}
        {#if claim.academic_session}
          <span class="meta-pill">{claim.academic_session}</span>
        {/if}
        {#if claim.semester}
          <span class="meta-pill">{claim.semester}</span>
        {/if}
      </div>
    </div>

    <div class="header-right">
      <button type="button" class="action-ghost-btn outline" onclick={handleCopyShareLink}>
        {#if isCopied}
          <Check size={12} color="var(--color-warm-cream)" />
          <span>Link Copied</span>
        {:else}
          <Share2 size={12} />
          <span>Share</span>
        {/if}
      </button>
      <div class="verified-pill">
        <CheckCircle2 size={12} />
        <span>Verified Attendance</span>
      </div>
    </div>
  </div>

  <!-- Big Title Area -->
  <div class="studio-title-area">
    <h1 class="session-main-title">{claim.session_title}</h1>
    <div class="session-sub-info">
      <span class="info-item">
        <Calendar size={13} /> Claimed on {claim.date}
      </span>
      {#if claim.session_code}
        <span class="info-divider">·</span>
        <span class="info-item code">
          Session Code: <strong>{claim.session_code}</strong>
        </span>
      {/if}
    </div>
  </div>

  <!-- Studio Sub-Navigation Tabs -->
  <div class="studio-nav-tabs" role="tablist" aria-label="Lecture studio tools">
    <button
      type="button"
      role="tab"
      class="tab-btn"
      class:active={activeTab === 'transcript'}
      aria-selected={activeTab === 'transcript'}
      onclick={() => (activeTab = 'transcript')}
    >
      <FileText size={14} />
      <span>Transcript</span>
    </button>
    <button
      type="button"
      role="tab"
      class="tab-btn"
      class:active={activeTab === 'chapters'}
      aria-selected={activeTab === 'chapters'}
      onclick={() => (activeTab = 'chapters')}
    >
      <Bookmark size={14} />
      <span>AI Chapters</span>
    </button>
    <button
      type="button"
      role="tab"
      class="tab-btn"
      class:active={activeTab === 'flashcards'}
      aria-selected={activeTab === 'flashcards'}
      onclick={() => (activeTab = 'flashcards')}
    >
      <Layers size={14} />
      <span>Flashcards</span>
    </button>
    <button
      type="button"
      role="tab"
      class="tab-btn"
      class:active={activeTab === 'audio'}
      aria-selected={activeTab === 'audio'}
      onclick={() => (activeTab = 'audio')}
    >
      <Volume2 size={14} />
      <span>Audio Replay</span>
    </button>
    <button
      type="button"
      role="tab"
      class="tab-btn"
      class:active={activeTab === 'notes'}
      aria-selected={activeTab === 'notes'}
      onclick={() => (activeTab = 'notes')}
    >
      <Edit3 size={14} />
      <span>Study Notes</span>
    </button>
  </div>

  <!-- Tab Content Canvas -->
  <div class="studio-content-body">
    {#if activeTab === 'transcript'}
      <TranscriptViewer transcript={transcriptText} />
    {:else if activeTab === 'chapters'}
      <ChapterBreakdown sessionId={claim.id} />
    {:else if activeTab === 'flashcards'}
      <FlashcardDeck />
    {:else if activeTab === 'audio'}
      <AudioPlayerPanel sessionCode={claim.session_code || claim.id} />
    {:else if activeTab === 'notes'}
      <LectureStudyNotes sessionId={claim.id} sessionTitle={claim.session_title} />
    {/if}
  </div>
</div>

<style>
  .lecture-studio-pane {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
    width: 100%;
  }

  .studio-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-12);
    padding-bottom: var(--spacing-12);
    border-bottom: 1px solid var(--color-cork-border);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
    flex-wrap: wrap;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .meta-tags-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .course-code-badge {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ember-accent);
    letter-spacing: 0.1em;
  }

  .course-title-sub {
    font-size: 12px;
    color: var(--color-driftwood);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .meta-pill {
    font-size: 10px;
    background: rgba(255, 237, 215, 0.08);
    color: var(--color-warm-cream-dim, #ffedd7cc);
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
  }

  .action-ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    padding: 5px 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .verified-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.1);
    border: 1px solid rgba(74, 222, 128, 0.2);
    padding: 4px 10px;
    border-radius: 4px;
    letter-spacing: 0.04em;
  }

  .studio-title-area {
    margin-top: 4px;
  }

  .session-main-title {
    font-family: var(--font-display);
    font-size: 26px;
    color: var(--color-warm-cream);
    margin: 0 0 6px 0;
    line-height: 1.25;
  }

  .session-sub-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-driftwood);
    flex-wrap: wrap;
  }

  .info-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .info-item.code strong {
    color: var(--color-warm-cream);
    font-family: var(--font-mono, monospace);
  }

  .info-divider {
    color: var(--color-cork-border);
  }

  .studio-nav-tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: 0;
    overflow-x: auto;
  }

  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 10px 16px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-weight: 500;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-driftwood);
    border-radius: 0;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.03);
  }

  .tab-btn.active {
    color: var(--color-warm-cream);
    border-bottom: 2px solid var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.06);
  }

  .studio-content-body {
    padding-top: var(--spacing-6);
  }

  @media (max-width: 768px) {
    .session-main-title {
      font-size: 22px;
    }
    .tab-btn {
      padding: 8px 12px;
      font-size: 11px;
    }
  }
</style>
