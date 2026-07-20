<script lang="ts">
  let {
    captionDraft = $bindable(""),
    apiNotice = "",
    onPublishCaption,
  }: {
    captionDraft: string;
    apiNotice?: string;
    onPublishCaption: () => void;
  } = $props();

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      onPublishCaption();
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
    onclick={onPublishCaption}
    disabled={!captionDraft.trim()}
  >
    Publish caption
  </button>
  {#if apiNotice}
    <p class="error">{apiNotice}</p>
  {/if}
</section>
