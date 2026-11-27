const { BASE, sleep, api, launch, phone, tap, type, marker } = require('./common.cjs');
const { execSync } = require('child_process');
(async () => {
  const live = require('./out/live_marks.json');
  const M = marker(__dirname + '/out/study_marks.json');
  const sid = execSync(`psql postgres://klasync:klasync-dev@127.0.0.1:5432/klasync -Atc "select id from lecture_sessions where short_code='${live.code}'"`).toString().trim();
  const tok = (await api('POST', '/auth/students/login', { email: 'ada@university.edu', password: 'demo-password-123' })).access_token;
  // Post-lecture AI jobs (chapters, flashcards, quiz, concept map) must be ready; this wait is cut from the video.
  const t0 = Date.now();
  for (;;) {
    const [ch, fc, qz, vz] = await Promise.all(['chapters', 'flashcards', 'quiz', 'visualizations'].map(k => api('GET', `/archive/sessions/${sid}/${k}`, null, tok).catch(() => [])));
    console.log('ready?', ch.length, fc.length, qz.length, vz.length);
    if (ch.length && fc.length && qz.length && vz.length) break;
    if (Date.now() - t0 > 600000) { console.log('timeout waiting for AI jobs'); break; }
    await sleep(5000);
  }
  const b = await launch(); const P = await phone(b, 'study_phone'); const p = P.page;
  const openTool = async (name) => {
    const tab = p.getByRole('tab', { name: new RegExp('^' + name, 'i') });
    if (await tab.count() && await tab.first().isVisible()) await tap(tab.first(), 900);
    else { await tap(p.locator('button[aria-haspopup=menu]').first(), 700); await tap(p.getByRole('menuitemradio', { name: new RegExp(name, 'i') }).first(), 900); }
  };
  M.mark('start');
  await p.goto(BASE + '/#/auth/student/login'); await sleep(1500);
  M.mark('signin_start');
  await type(p.locator('input[type=email]'), 'ada@university.edu', 90);
  await type(p.locator('input[type=password]'), 'demo-password-123', 90);
  await sleep(300); await p.locator('input[type=password]').press('Enter');
  await p.waitForSelector('text=Lecture archive'); await sleep(2500);
  M.mark('archive_start');
  await p.mouse.move(0, 0); await sleep(1500);
  await tap(p.locator('text=Attended').first(), 3000);
  M.mark('transcript_start');
  await sleep(2500);
  await tap(p.getByRole('button', { name: /^play/i }).first(), 8000).catch(() => console.log('no play btn'));
  await tap(p.getByRole('button', { name: /^pause/i }).first(), 800).catch(() => {});
  M.mark('chapters_start');
  await openTool('Chapters'); await sleep(6000);
  M.mark('summary_start');
  await openTool('Summary'); await sleep(6000);
  // concept map: the canvas sits below the controls, so scroll it into view, then explore
  M.mark('map_start');
  await openTool('Map'); await sleep(1500);
  const canvas = () => p.locator('[aria-label^="Concept map of"]').first();
  const toCanvas = () => canvas().evaluate(e => e.scrollIntoView({ block: 'start', behavior: 'smooth' }));
  const toTop = () => p.locator('.map-pane').evaluate(e => e.scrollTo({ top: 0, behavior: 'smooth' }));
  await toCanvas(); await sleep(7000);              // 3D map drifting
  await toTop(); await sleep(900);
  await tap(p.getByRole('button', { name: /^\s*2D/ }).first(), 700); await toCanvas(); await sleep(4500); // 2D graph
  await toTop(); await sleep(900);
  await tap(p.getByRole('button', { name: /tour/i }).first(), 500); await toCanvas(); await sleep(7000);   // guided tour
  M.mark('cards_start');
  await openTool('Cards'); await sleep(2500);
  for (let i = 0; i < 4; i++) {
    await tap(p.getByRole('button', { name: /show answer/i }).first(), 2600);
    await tap(p.getByRole('button', { name: i === 1 ? /still learning/i : /got it/i }).first(), 1500);
  }
  M.mark('quiz_start');
  await openTool('Quiz'); await sleep(2500);
  for (let i = 0; i < 8; i++) {
    const opts = p.locator('[data-testid=quiz-option]');
    if (!(await opts.count())) break;
    await tap(opts.nth(i % 3 === 2 ? 0 : 1), 1100);
    await sleep(2200);
    await tap(p.getByRole('button', { name: /^(next|see results)$/i }).first(), 700);
    if (await p.locator('[data-testid=quiz-result]').count()) break;
  }
  await sleep(3500);
  M.mark('ask_start');
  await openTool('Ask'); await sleep(1200);
  await type(p.getByLabel('Ask a question about this lecture'), 'Where does the Calvin cycle take place and what does it need?', 55);
  await sleep(300); await tap(p.getByRole('button', { name: /send question/i }), 500);
  await p.waitForSelector('text=/stroma/i', { timeout: 60000 }).catch(() => console.log('no stroma answer'));
  await sleep(6000); M.mark('study_end');
  await p.screenshot({ path: __dirname + '/out/study_end.png' });
  const vid = await p.video().path();
  M.save({ vid, sid }); await P.ctx.close(); await b.close(); console.log('done', vid);
})().catch(e => { console.error(e); process.exit(1); });
