<script lang="ts">
  let { onClick }: { onClick: () => void } = $props();
</script>

<button class="brand app-no-drag" onclick={onClick} aria-label="Klasync home">
  <svg
    class="brand-mark"
    aria-hidden="true"
    viewBox="0 0 48 48"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="M8 26 A18 18 0 0 1 40 26"
      fill="none"
      stroke="#dc5000"
      stroke-width="2.5"
      stroke-linecap="round"
    />
    <text
      x="9"
      y="40"
      font-family="'Playfair Display', Georgia, serif"
      font-size="28"
      font-weight="500"
      fill="#ffedd7">K</text
    >
    <g fill="#ffedd7" opacity="0.75">
      <rect x="30" y="28" width="2" height="8" rx="1" /><rect
        x="33.5"
        y="24"
        width="2"
        height="12"
        rx="1"
      /><rect x="37" y="26" width="2" height="10" rx="1" /><rect
        x="40.5"
        y="29"
        width="2"
        height="6"
        rx="1"
      />
    </g>
  </svg>
  <span class="brand-name">Klasync</span>
</button>

<style>
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    margin: 0;
    min-height: 0 !important;
    min-width: 0 !important;
  }
  .brand-mark {
    width: 32px;
    height: 32px;
    transform: translateY(-2px);
    flex-shrink: 0;
  }
  .brand-name {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 500;
    letter-spacing: 0.05em;
    color: var(--color-warm-cream);
    text-transform: uppercase;
    line-height: 1;
  }
</style>
