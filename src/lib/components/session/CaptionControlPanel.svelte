<script lang="ts">
  import CaptionTransmitterCard from './CaptionTransmitterCard.svelte';
  import CaptionHistoryCard from './CaptionHistoryCard.svelte';

  let {
    captionDraft = $bindable(''),
    captions = [],
    apiNotice = '',
    onPublishCaption,
  }: {
    captionDraft: string;
    captions?: string[];
    apiNotice?: string;
    onPublishCaption: () => Promise<void> | void;
  } = $props();

  let isPublishing = $state(false);

  async function handlePublish() {
    if (!captionDraft.trim()) return;
    isPublishing = true;
    try {
      await onPublishCaption();
    } finally {
      isPublishing = false;
    }
  }
</script>

<section class="caption-control-wrap">
  <CaptionTransmitterCard bind:captionDraft {apiNotice} {isPublishing} onPublishCaption={handlePublish} />
  <CaptionHistoryCard {captions} />
</section>

<style>
  .caption-control-wrap { display: flex; flex-direction: column; gap: var(--spacing-16); }
</style>
