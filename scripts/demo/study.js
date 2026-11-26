const { BASE, sleep, api, launch, phone, tap, type, marker } = require('./common');
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
  await type(p.locator('input[type=email]'), 'ada@university.edu', 55);
  await type(p.locator('input[type=password]'), 'demo-password-123', 55);
  await sleep(300); await p.locator('input[type=password]').press('Enter');
  await p.waitForSelector('text=Lecture archive'); await sleep(3000); M.mark('archive');
  await p.mouse.move(0, 0);
  await tap(p.locator('text=Attended').first(), 3500); M.mark('studio_open');
  // transcript + audio
  await sleep(2500);
  await tap(p.getByRole('button', { name: /^play/i }).first(), 6000).catch(() => console.log('no play btn')); M.mark('audio_played');
  await tap(p.getByRole('button', { name: /^pause/i }).first(), 800).catch(() => {});
  await openTool('Chapters'); await sleep(3500); M.mark('chapters');
  await openTool('Summary'); await sleep(4500); M.mark('summary');
  await openTool('Map'); await sleep(6000); M.mark('map');
  // flashcards
  await openTool('Cards'); await sleep(2000); M.mark('cards');
  for (let i = 0; i < 3; i++) {
    await tap(p.getByRole('button', { name: /show answer/i }).first(), 1800);
    await tap(p.getByRole('button', { name: i === 1 ? /still learning/i : /got it/i }).first(), 1200);
  }
  M.mark('cards_done');
  // quiz
  await openTool('Quiz'); await sleep(2500); M.mark('quiz');
  for (let i = 0; i < 8; i++) {
    const opts = p.locator('[data-testid=quiz-option]');
    if (!(await opts.count())) break;
    await tap(opts.nth(i % 3 === 2 ? 0 : 1), 1100); // mostly picks option B
    await sleep(1500);
    await tap(p.getByRole('button', { name: /^(next|see results)$/i }).first(), 700);
    if (i === 2) M.mark('quiz_midway');
    if (await p.locator('[data-testid=quiz-result]').count()) break;
  }
  await sleep(3500); M.mark('quiz_result');
  // ask
  await openTool('Ask'); await sleep(1200);
  await type(p.getByLabel('Ask a question about this lecture'), 'Where does the Calvin cycle take place and what does it need?', 40);
  await sleep(300); await tap(p.getByRole('button', { name: /send question/i }), 500); M.mark('ask_sent');
  await p.waitForSelector('text=/stroma/i', { timeout: 60000 }).catch(() => console.log('no stroma answer'));
  await sleep(4500); M.mark('ask_answer');
  await p.screenshot({ path: __dirname + '/out/study_end.png' });
  const vid = await p.video().path();
  M.save({ vid, sid }); await P.ctx.close(); await b.close(); console.log('done', vid);
})().catch(e => { console.error(e); process.exit(1); });
