<script lang="ts">
  const features = [
    {
      num: "01",
      title: "Accessibility First",
      desc: "Live captions, digestible audio streams, and keyboard-navigable UI eliminate physical, sensory, and cognitive barriers in lecture halls."
    },
    {
      num: "02",
      title: "Searchable Archive",
      desc: "Lectures transform into structured summaries, real-time transcripts, and downloadable study materials that outlast the classroom session."
    },
    {
      num: "03",
      title: "Fair Attendance",
      desc: "Authenticated guest participation verifies presence against rosters seamlessly, ensuring fair compliance without forcing initial account creation."
    }
  ];
</script>

<section class="feature-grid">
  {#each features as feat}
    <article>
      <span class="feat-num">{feat.num}</span>
      <h2>{feat.title}</h2>
      <p>{feat.desc}</p>
    </article>
  {/each}
</section>
