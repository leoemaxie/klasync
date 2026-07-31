<script lang="ts">
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";

  let {
    captionDraft = $bindable(""),
    apiNotice = "",
    onPublishCaption,
  }: {
    captionDraft: string;
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

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      handlePublish();
    }
  }
</script>

<section class="caption-control">
  <p class="eyebrow">CAPTION TRANSMITTER</p>
  <label>
    Broadcast a caption chunk
    <input
      bind:value={captionDraft}
      placeholder="Type a caption for students"
      onkeydown={handleKeyDown}
    />
  </label>
  <button
    class="outline"
    onclick={handlePublish}
    disabled={!captionDraft.trim() || isPublishing}
  >
    {#if isPublishing}
      <ButtonSpinner label="Broadcasting caption..." /> Publishing...
    {:else}
      Publish caption
    {/if}
  </button>
  {#if apiNotice}
    <p class="error">{apiNotice}</p>
  {/if}
</section>
