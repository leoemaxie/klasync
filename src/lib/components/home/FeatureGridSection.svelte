<script lang="ts">
  const features = [
    {
      num: '01',
      title: 'Accessibility First',
      desc: 'Live captions, audio streams, and keyboard navigation for all students.',
    },
    {
      num: '02',
      title: 'Searchable Archive',
      desc: 'Searchable transcripts, structured summaries, and study flashcards.',
    },
    {
      num: '03',
      title: 'Fair Attendance',
      desc: 'Verified student presence against course rosters without required accounts.',
    },
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
