<script lang="ts">
  let { value = 'KLASYNC-ROOM', size = 128 }: { value: string; size?: number } =
    $props();

  const matrixSize = 21;
  const pathData = $derived.by(() => {
    const cells: boolean[][] = Array.from({ length: matrixSize }, () =>
      Array(matrixSize).fill(false)
    );

    const addFinder = (r: number, c: number) => {
      for (let i = 0; i < 7; i++) {
        for (let j = 0; j < 7; j++) {
          if (
            i === 0 ||
            i === 6 ||
            j === 0 ||
            j === 6 ||
            (i >= 2 && i <= 4 && j >= 2 && j <= 4)
          ) {
            cells[r + i][c + j] = true;
          }
        }
      }
    };
    addFinder(0, 0);
    addFinder(0, matrixSize - 7);
    addFinder(matrixSize - 7, 0);

    let hash = 0;
    for (let i = 0; i < value.length; i++)
      hash = (hash << 5) - hash + value.charCodeAt(i);
    for (let r = 0; r < matrixSize; r++) {
      for (let c = 0; c < matrixSize; c++) {
        if (
          !cells[r][c] &&
          (r > 7 || c > 7) &&
          (r < 14 || c > 7) &&
          (r > 7 || c < 14)
        ) {
          cells[r][c] = (r * 31 + c * 17 + Math.abs(hash)) % 3 === 0;
        }
      }
    }

    let d = '';
    for (let r = 0; r < matrixSize; r++) {
      for (let c = 0; c < matrixSize; c++) {
        if (cells[r][c]) {
          d += `M${c},${r}h1v1h-1z `;
        }
      }
    }
    return d;
  });
</script>

<div class="qr-svg-container" style="--size: {size}px;">
  <svg
    viewBox="0 0 21 21"
    xmlns="http://www.w3.org/2000/svg"
    class="qr-svg"
    role="img"
    aria-label="QR Code for {value}"
  >
    <rect width="21" height="21" fill="#ffedd7" rx="1.5" />
    <path d={pathData} fill="#100904" />
  </svg>
</div>

<style>
  .qr-svg-container {
    width: var(--size);
    height: var(--size);
    padding: 8px;
    background: var(--color-warm-cream);
    border-radius: var(--radius-cards);
    display: inline-block;
  }
  .qr-svg {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
