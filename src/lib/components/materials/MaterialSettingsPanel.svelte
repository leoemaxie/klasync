<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import {
    fetchMaterialCapabilities,
    fetchMaterialSettings,
    fetchPluginSettings,
    resetPluginSetting,
    saveMaterialSettings,
    setPluginSetting,
    type MaterialCapabilities,
    type MaterialSettings,
    type PluginKind,
    type PluginSetting,
  } from '$lib/api/materials';

  let { courseId }: { courseId: string } = $props();

  let settings = $state<MaterialSettings | null>(null);
  let caps = $state<MaterialCapabilities | null>(null);
  let plugins = $state<PluginSetting[]>([]);
  let loading = $state(true);
  let hookUrl = $state('');
  let hookSecret = $state('');
  let saving = $state(false);
  let error = $state('');
  let notice = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      [settings, caps, { plugins }] = await Promise.all([
        fetchMaterialSettings(courseId),
        fetchMaterialCapabilities(),
        fetchPluginSettings(courseId),
      ]);
      hookUrl =
        (plugins.find((x) => x.id === 'webhook')?.config.url as string) ?? '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load settings';
    } finally {
      loading = false;
    }
  }
  onMount(() => void load());

  async function save() {
    if (!settings) return;
    saving = true;
    error = '';
    notice = '';
    try {
      settings = await saveMaterialSettings(courseId, settings);
      notice = 'Settings saved.';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save';
    } finally {
      saving = false;
    }
  }

  async function toggle(p: PluginSetting, enabled: boolean) {
    error = '';
    try {
      await setPluginSetting(courseId, p.kind, p.id, { enabled });
      plugins = plugins.map((x) =>
        x.kind === p.kind && x.id === p.id
          ? { ...x, enabled, source: 'course' }
          : x
      );
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not update plug-in';
    }
  }

  async function saveHook(p: PluginSetting) {
    error = '';
    notice = '';
    try {
      await setPluginSetting(courseId, p.kind, p.id, {
        enabled: true,
        config: {
          url: hookUrl.trim(),
          ...(hookSecret ? { secret: hookSecret } : {}),
        },
      });
      hookSecret = '';
      plugins = (await fetchPluginSettings(courseId)).plugins;
      notice = 'Webhook saved.';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save the webhook';
    }
  }

  async function reset(p: PluginSetting) {
    error = '';
    try {
      await resetPluginSetting(courseId, p.kind, p.id);
      const fresh = await fetchPluginSettings(courseId);
      plugins = fresh.plugins;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not reset plug-in';
    }
  }

  const groups: { kind: PluginKind; title: string; hint: string }[] = [
    {
      kind: 'generator',
      title: 'Content generators',
      hint: 'Parts of the handout the AI writes.',
    },
    {
      kind: 'renderer',
      title: 'Export formats',
      hint: 'Files students and you can download.',
    },
    {
      kind: 'delivery',
      title: 'Delivery',
      hint: 'How published material reaches students.',
    },
    {
      kind: 'analyzer',
      title: 'Analysis',
      hint: 'Reports built from your materials.',
    },
  ];
  const sourceLabel = {
    course: 'this course',
    global: 'institution',
    default: 'default',
  };
</script>

{#if loading}
  <div class="center"><Spinner /></div>
{:else if settings}
  <form
    class="settings"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <h3>Handout defaults</h3>
    <label class="check">
      <input type="checkbox" bind:checked={settings.auto_generate} />
      Draft a handout automatically after each lecture
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={settings.seed_from_previous} />
      Start from the previous offering's handout when one exists
    </label>
    <div class="grid">
      <label class="field">
        <span>Depth</span>
        <select bind:value={settings.depth}>
          <option value="concise">Concise</option>
          <option value="standard">Standard</option>
          <option value="detailed">Detailed</option>
        </select>
      </label>
      <label class="field">
        <span>Audience level</span>
        <input
          type="text"
          bind:value={settings.audience_level}
          maxlength="60"
        />
      </label>
      <label class="field">
        <span>Language</span>
        <input type="text" bind:value={settings.language} maxlength="40" />
      </label>
      <label class="field">
        <span>Tone</span>
        <input type="text" bind:value={settings.tone} maxlength="60" />
      </label>
      <label class="field">
        <span>Template</span>
        <select bind:value={settings.template}>
          {#each caps?.templates ?? [] as t}<option value={t.id}>
              {t.name}
            </option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <Button type="submit" variant="accent" disabled={saving}>
        {saving ? 'Saving…' : 'Save settings'}
      </Button>
      {#if notice}<span class="ok" role="status">{notice}</span>{/if}
    </div>
  </form>

  <section class="settings">
    <h3>Plug-ins for this course</h3>
    <p class="muted">
      Turn parts of the pipeline on or off for this course only. Anything you
      don't change follows the institution default.
    </p>
    {#each groups as g}
      {@const items = plugins.filter((p) => p.kind === g.kind)}
      {#if items.length}
        <div class="group">
          <h4>{g.title} <span class="muted">· {g.hint}</span></h4>
          {#each items as p (p.kind + p.id)}
            <div class="plugin">
              <label class="check">
                <input
                  type="checkbox"
                  checked={p.enabled}
                  disabled={p.protected}
                  onchange={(e) => void toggle(p, e.currentTarget.checked)}
                />
                {p.label}
              </label>
              <span class="muted">
                {p.protected ? 'always on' : `set by ${sourceLabel[p.source]}`}
              </span>
              {#if p.source === 'course'}
                <Button size="sm" variant="ghost" onclick={() => void reset(p)}>
                  Use default
                </Button>
              {/if}
            </div>
            {#if p.kind === 'delivery' && p.id === 'webhook'}
              <div class="webhook">
                <label class="field">
                  <span>Webhook URL (https)</span>
                  <input
                    type="url"
                    placeholder="https://lms.example.edu/hooks/klasync"
                    bind:value={hookUrl}
                  />
                </label>
                <label class="field">
                  <span>
                    Signing secret {p.config.has_secret
                      ? '(set — leave blank to keep)'
                      : '(optional)'}
                  </span>
                  <input
                    type="password"
                    autocomplete="off"
                    bind:value={hookSecret}
                  />
                </label>
                <div class="row">
                  <Button
                    size="sm"
                    variant="outline"
                    onclick={() => void saveHook(p)}
                  >
                    Save webhook
                  </Button>
                  <span class="muted">
                    Each publish can push a signed JSON message with 24-hour
                    download links. Verify it with the
                    <code>X-Klasync-Signature</code> header (HMAC-SHA256).
                  </span>
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    {/each}
  </section>
{/if}
{#if error}<p class="err" role="alert">{error}</p>{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .settings {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    margin-bottom: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
  }
  h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 16px;
  }
  h4 {
    margin: 8px 0 4px;
    font-size: 13px;
    color: var(--text-primary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }
  input[type='text'],
  select {
    padding: 7px 9px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .plugin {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
    padding: 2px 0;
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    font-weight: 400;
    color: var(--text-tertiary);
  }
  .ok {
    color: #7fd3a0;
    font-size: 13px;
  }
  .err {
    margin: 0;
    padding: 8px 12px;
    border: 1px solid #ff9b7a;
    border-radius: 8px;
    color: #ff9b7a;
    font-size: 13px;
  }
  /* The global label/input styles are for the sign-in forms; undo them here. */
  .check {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-secondary);
  }
  .check input[type='checkbox'] {
    width: auto;
    margin: 0;
    flex: none;
  }
</style>
