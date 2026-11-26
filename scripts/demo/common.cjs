const { chromium, devices } = require('playwright');
const fs = require('fs');
const BASE = 'http://127.0.0.1:1420';
const API = 'http://127.0.0.1:8787/api/v1';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function api(m, path, body, tok) {
  const r = await fetch(API + path, { method: m, headers: { 'content-type': 'application/json', ...(tok ? { authorization: 'Bearer ' + tok } : {}) }, body: body ? JSON.stringify(body) : undefined });
  const t = await r.text();
  if (r.status >= 300) throw new Error(`${m} ${path} ${r.status} ${t.slice(0, 200)}`);
  return t ? JSON.parse(t) : null;
}
// On-screen pointer + tap ripple so viewers can follow the interaction.
const OVERLAY = `
(() => {
  const add = () => {
    if (document.getElementById('__cur')) return;
    const s = document.createElement('style');
    s.textContent = '#__cur{position:fixed;z-index:2147483647;width:22px;height:22px;border-radius:50%;background:rgba(255,140,60,.35);border:2px solid #ff8c3c;pointer-events:none;transform:translate(-50%,-50%);transition:transform .08s;display:none}#__cur.on{display:block}.__rip{position:fixed;z-index:2147483647;width:14px;height:14px;border-radius:50%;border:3px solid #ff8c3c;pointer-events:none;transform:translate(-50%,-50%);animation:__r .55s ease-out forwards}@keyframes __r{to{width:70px;height:70px;opacity:0}}';
    document.documentElement.appendChild(s);
    const c = document.createElement('div'); c.id = '__cur'; document.documentElement.appendChild(c);
    addEventListener('mousemove', e => { c.classList.add('on'); c.style.left = e.clientX + 'px'; c.style.top = e.clientY + 'px'; }, true);
    addEventListener('pointerdown', e => { const r = document.createElement('div'); r.className = '__rip'; r.style.left = e.clientX + 'px'; r.style.top = e.clientY + 'px'; document.documentElement.appendChild(r); setTimeout(() => r.remove(), 600); }, true);
  };
  if (document.documentElement) add(); else addEventListener('DOMContentLoaded', add);
})();`;
async function launch() {
  return chromium.launch({ args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream', '--use-file-for-fake-audio-capture=' + __dirname + '/lecture48.wav%noloop', '--autoplay-policy=no-user-gesture-required'] });
}
async function desktop(b, name, extra = {}) {
  const ctx = await b.newContext({ viewport: { width: 1280, height: 800 }, permissions: ['microphone'], recordVideo: { dir: __dirname + '/out/' + name, size: { width: 1280, height: 800 } }, ...extra });
  await ctx.addInitScript(OVERLAY);
  const t0 = Date.now(); const page = await ctx.newPage();
  page.on('pageerror', (e) => console.log(`[${name}] pageerror`, e.message));
  return { ctx, page, t0 };
}
async function phone(b, name, extra = {}) {
  const d = devices['iPhone 14'];
  const ctx = await b.newContext({ ...d, recordVideo: { dir: __dirname + '/out/' + name, size: { width: d.viewport.width * 2, height: d.viewport.height * 2 } }, ...extra });
  await ctx.addInitScript(OVERLAY);
  const t0 = Date.now(); const page = await ctx.newPage();
  page.on('pageerror', (e) => console.log(`[${name}] pageerror`, e.message));
  return { ctx, page, t0 };
}
// Move the pointer to the element like a person would, then click.
async function click(page, loc, pause = 350) {
  await loc.scrollIntoViewIfNeeded().catch(() => {});
  const box = await loc.boundingBox();
  if (box) { await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 18 }); await sleep(180); }
  await loc.click(); await sleep(pause);
}
async function tap(loc, pause = 500) { await loc.scrollIntoViewIfNeeded().catch(() => {}); await loc.tap(); await sleep(pause); }
async function type(loc, text, delay = 45) { await loc.click(); await loc.pressSequentially(text, { delay }); }
function marker(file) {
  const marks = [];
  const T0 = Date.now();
  return { mark(name, extra = {}) { marks.push({ name, ms: Date.now() - T0, ...extra }); console.log(`[mark] ${name} +${((Date.now() - T0) / 1000).toFixed(1)}s`); }, T0, save(extra = {}) { fs.writeFileSync(file, JSON.stringify({ T0, marks, ...extra }, null, 1)); } };
}
module.exports = { BASE, API, sleep, api, launch, desktop, phone, click, tap, type, marker, devices };
