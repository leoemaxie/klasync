<script lang="ts">
  import '../styles/theme.css';
  import '../styles/layout.css';
  import '../styles/elements.css';
  import '../styles/forms.css';
  import '../styles/components.css';

  let { children } = $props();

  function handleContextMenu(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    const isEditable =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable ||
      target.closest('.selectable-text');
    if (!isEditable) {
      e.preventDefault();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    // ESC key closes popups / modals
    if (e.key === 'Escape') {
      const activeModal = document.querySelector(
        '.modal-backdrop, [role="dialog"]'
      ) as HTMLElement | null;
      if (activeModal) {
        activeModal.click();
      }
    }
  }
</script>

<svelte:window oncontextmenu={handleContextMenu} onkeydown={handleKeyDown} />

<svelte:head>
  <title>Klasync — Every lecture, within reach</title>
  <meta
    name="description"
    content="Live captions, fair attendance, and a lasting learning archive — without making students create an account just to join class."
  />
</svelte:head>

{@render children()}
