<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { X } from '@lucide/svelte';

  let { onClose }: { onClose: () => void } = $props();

  const groups = [
    {
      title: 'Playback',
      items: [
        ['Space / K', 'Play or pause'],
        ['J / L', 'Back / forward 10 seconds'],
        ['← / →', 'Back / forward 5 seconds'],
        ['[ / ]', 'Previous / next chapter'],
        ['< / >', 'Slower / faster'],
        ['B', 'Save this moment'],
      ],
    },
    {
      title: 'Workspace',
      items: [
        ['1 – 7', 'Open Transcript, Notes, Ask, Summary, Map, Cards, Chapters'],
        ['N', 'Add a note at this moment'],
        ['/', 'Ask the lecture'],
        ['F', 'Focus mode'],
        ['?', 'This cheat sheet'],
      ],
    },
  ];

  let dialog: HTMLDivElement | undefined = $state();
  let previous: Element | null = null;

  onMount(() => {
    previous = document.activeElement;
    void tick().then(() =>
      dialog?.querySelector<HTMLElement>('button')?.focus()
    );
    return () => (previous as HTMLElement | null)?.focus?.();
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.stopPropagation();
      onClose();
      return;
    }
    // Keep Tab inside the dialog.
    if (event.key === 'Tab' && dialog) {
      const focusable = dialog.querySelectorAll<HTMLElement>(
        'button, [href], input, [tabindex]:not([tabindex="-1"])'
      );
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }
</script>

<!-- The shared layout handler clicks `.modal-backdrop` on Escape. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="modal-backdrop scrim"
  onclick={(e) => e.target === e.currentTarget && onClose()}
>
  <div
    class="dialog"
    bind:this={dialog}
    role="dialog"
    aria-modal="true"
    aria-labelledby="shortcuts-title"
    tabindex="-1"
    onkeydown={onKeydown}
  >
    <header>
      <h2 id="shortcuts-title">Keyboard shortcuts</h2>
      <button type="button" class="close" aria-label="Close" onclick={onClose}>
        <X size={18} />
      </button>
    </header>
    <div class="cols">
      {#each groups as group}
        <section>
          <h3>{group.title}</h3>
          <dl>
            {#each group.items as [keys, what]}
              <div class="row">
                <dt>
                  {#each keys.split(' / ') as k, i}
                    {#if i > 0}<span class="or">or</span>{/if}<kbd>{k}</kbd>
                  {/each}
                </dt>
                <dd>{what}</dd>
              </div>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
    <p class="foot">Shortcuts pause while you are typing in a field.</p>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(6, 3, 1, 0.7);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    animation: fade var(--motion-base) var(--ease-out);
  }
  .dialog {
    width: min(640px, 100%);
    max-height: min(560px, 92dvh);
    overflow-y: auto;
    padding: 22px 24px 18px;
    background: var(--surface-3);
    background-color: #241408;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    box-shadow: var(--elev-3);
    animation: pop var(--motion-slow) var(--ease-out);
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 22px;
  }
  .close {
    display: grid;
    place-items: center;
    width: 36px;
    min-width: 36px;
    height: 36px;
    min-height: 36px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-secondary);
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .close:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .cols {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px 28px;
  }
  h3 {
    margin: 0 0 8px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }
  dl {
    margin: 0;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(84px, auto) minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    padding: 7px 0;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 13px;
  }
  dt {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  dd {
    margin: 0;
    color: var(--text-secondary);
  }
  kbd {
    display: inline-block;
    min-width: 24px;
    padding: 2px 7px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 6px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    text-align: center;
  }
  .or {
    font-size: 11px;
    color: var(--text-tertiary);
  }
  .foot {
    margin: 14px 0 0;
    font-size: 12px;
    color: var(--text-tertiary);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }
  @media (max-width: 620px) {
    .cols {
      grid-template-columns: 1fr;
    }
  }
</style>
