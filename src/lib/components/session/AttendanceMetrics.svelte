<script lang="ts">
  import { Users, ShieldCheck, UserX, Activity } from "@lucide/svelte";

  let {
    total = 0,
    verified = 0,
    provisional = 0,
    pulses = 0,
    matchRate = 0
  }: {
    total?: number;
    verified?: number;
    provisional?: number;
    pulses?: number;
    matchRate?: number;
  } = $props();
</script>

<div class="metrics-grid">
  <div class="metric-card">
    <div class="metric-icon-box"><Users size={16} /></div>
    <div class="metric-info">
      <span class="m-val">{total}</span>
      <span class="m-lbl">Total Joined</span>
    </div>
  </div>

  <div class="metric-card verified-card">
    <div class="metric-icon-box v-icon"><ShieldCheck size={16} /></div>
    <div class="metric-info">
      <div class="val-row">
        <span class="m-val v-val">{verified}</span>
        <span class="rate-badge">{matchRate}% match</span>
      </div>
      <span class="m-lbl">Roster Verified</span>
    </div>
  </div>

  <div class="metric-card provisional-card">
    <div class="metric-icon-box p-icon"><UserX size={16} /></div>
    <div class="metric-info">
      <span class="m-val p-val">{provisional}</span>
      <span class="m-lbl">Provisional Guests</span>
    </div>
  </div>

  <div class="metric-card pulses-card">
    <div class="metric-icon-box pulse-icon"><Activity size={16} /></div>
    <div class="metric-info">
      <span class="m-val pulse-val">{pulses}</span>
      <span class="m-lbl">Active Check-in Pulses</span>
    </div>
  </div>
</div>

<style>
  .metrics-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 12px; }
  .metric-card { display: flex; align-items: center; gap: 12px; padding: 12px 14px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; }
  .metric-icon-box { width: 34px; height: 34px; border-radius: 6px; background: rgba(255, 255, 255, 0.05); border: 1px solid var(--color-cork-border); display: flex; align-items: center; justify-content: center; color: var(--color-warm-cream); flex-shrink: 0; }
  .v-icon { color: #4ab772; background: rgba(74, 183, 114, 0.12); border-color: rgba(74, 183, 114, 0.3); }
  .p-icon { color: #e5a93c; background: rgba(229, 169, 60, 0.12); border-color: rgba(229, 169, 60, 0.3); }
  .pulse-icon { color: #38bdf8; background: rgba(56, 189, 248, 0.12); border-color: rgba(56, 189, 248, 0.3); }
  .metric-info { display: flex; flex-direction: column; flex: 1; }
  .val-row { display: flex; align-items: center; justify-content: space-between; }
  .m-val { font-family: var(--font-display); font-size: 20px; font-weight: 700; color: var(--color-warm-cream); line-height: 1.1; }
  .v-val { color: #4ab772; }
  .p-val { color: #e5a93c; }
  .pulse-val { color: #38bdf8; }
  .rate-badge { font-size: 9px; font-weight: 700; color: #4ab772; background: rgba(74, 183, 114, 0.15); padding: 1px 6px; border-radius: 4px; }
  .m-lbl { font-size: 9px; letter-spacing: 0.08em; text-transform: uppercase; color: var(--color-warm-cream-dim); margin-top: 2px; }
  @media (max-width: 900px) { .metrics-grid { grid-template-columns: 1fr 1fr; } }
</style>
