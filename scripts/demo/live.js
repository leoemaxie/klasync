const { BASE, sleep, api, launch, desktop, phone, click, tap, type, marker } = require('./common');
(async () => {
  const M = marker(__dirname + '/out/live_marks.json');
  const b = await launch();
  const D = await desktop(b, 'live_desktop'); const P = await phone(b, 'live_phone');
  const d = D.page, p = P.page;
  const off = { desktop: D.t0 - M.T0, phone: P.t0 - M.T0 };
  M.mark('start');
  // 1. Lecturer signs in
  await d.goto(BASE + '/#/auth/lecturer/login'); await sleep(1500);
  M.mark('login_page');
  await type(d.locator('input[type=email]'), 'amara.eze@university.edu');
  await type(d.locator('input[type=password]'), 'demo-password-123');
  await sleep(400); await d.locator('input[type=password]').press('Enter');
  await d.waitForSelector('text=COURSE'); await sleep(3500);
  M.mark('course_tab');
  // roster, then mic/device, then room
  await d.mouse.wheel(0, 500); await sleep(2500); await d.mouse.wheel(0, -500); await sleep(500);
  await click(d, d.locator('button:has-text("DEVICE")'), 2500); M.mark('device_tab');
  await click(d, d.locator('button:has-text("ROOM")'), 1500);
  await click(d, d.locator('button:has-text("START SESSION")'), 2500);
  const code = (await d.locator('body').innerText()).match(/LIVE · ([A-Z0-9]{8})/)[1];
  M.mark('session_started', { code }); await sleep(3000);
  // 2. Student phone joins with the code
  await p.goto(BASE + '/#/join'); await sleep(1500);
  M.mark('phone_join_page');
  await type(p.locator('input[placeholder="e.g. A4K9QZ"]'), code, 90);
  await type(p.locator('input[placeholder="MAT/2023/001"]'), 'BIO/26/001', 70);
  await type(p.locator('input[placeholder="Ada Okafor"]'), 'Ada Okafor', 70);
  await sleep(400); await tap(p.getByRole('button', { name: /join session/i }).last(), 2500);
  M.mark('phone_joined');
  // classmates join in the background (simulated phones)
  const joined = [];
  const others = async () => { for (let i = 2; i <= 11; i++) { const j = await api('POST', `/sessions/code/${code}/join`, { matric_number: `BIO/26/${String(i).padStart(3, '0')}`, display_name: ['', 'Ada Okafor', 'Bola Adeyemi', 'Chidi Nwosu', 'Dayo Balogun', 'Efe Eze', 'Funke Ojo', 'Gbenga Lawal', 'Halima Musa', 'Ife Bello', 'Jide Umar', 'Kemi Ayoola', 'Lara Obi'][i] }); joined.push(j); await sleep(900); } };
  const othersP = others();
  // 3. Lecturer starts the mic: the TTS lecture now plays as the room audio
  await sleep(2000);
  await click(d, d.locator('button:has-text("Record Audio")'), 800);
  const audioT = Date.now(); M.mark('audio_start');
  const at = () => (Date.now() - audioT) / 1000;
  const waitAt = async (s) => { while (at() < s) await sleep(200); };
  await othersP;
  // heartbeats for everyone so attendance scores are real
  const presence = [1, 1, 0.9, 0.8, 0.7, 0.6, 1, 0.4, 0.85, 0.95];
  const beat = setInterval(() => joined.forEach((j, i) => { if (Math.random() < presence[i % presence.length]) api('POST', `/participants/${j.id}/heartbeat`).catch(() => {}); }), 25000);
  // attendance roll builds up on the lecturer side
  await waitAt(12); await click(d, d.locator('button:has-text("ATTENDANCE")'), 800);
  await click(d, d.locator('button[aria-label*="efresh"], button:has(svg.lucide-refresh-cw)').first(), 1500); M.mark('attendance_view');
  // student checks in + raises a hand
  await waitAt(24); await tap(p.getByRole('button', { name: /check in/i }).first(), 1200); M.mark('phone_checkin');
  await click(d, d.locator('button:has-text("ROOM")'), 1000);
  await waitAt(34); await tap(p.getByRole('button', { name: /raise hand/i }).first(), 1500); M.mark('phone_hand');
  await d.mouse.wheel(0, 700); await sleep(1500); M.mark('lect_hand_view');
  // student asks a question
  await waitAt(48);
  await type(p.locator('input[placeholder^="Ask a question"], textarea[placeholder^="Ask a question"]').first(), 'What does Rubisco actually do?', 55);
  await sleep(300); await tap(p.getByRole('button', { name: /^ask$/i }).first(), 2000); M.mark('phone_question');
  await d.mouse.wheel(0, 700); await sleep(2500); M.mark('lect_question_view');
  // rolling summary on both sides
  await waitAt(75); await p.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' })); await sleep(2000);
  await d.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' })); await sleep(1500);
  await d.mouse.wheel(0, 500); M.mark('summary_view'); await sleep(4000);
  // lecturer launches an AI quiz from what was just said
  await waitAt(100);
  await d.evaluate(() => window.scrollTo({ top: 300, behavior: 'smooth' })); await sleep(1000);
  await click(d, d.locator('button:has-text("AI quiz"), button:has-text("AI QUIZ")').first(), 500); M.mark('ai_quiz_click');
  await d.locator('.live-poll-panel article.poll').waitFor({ timeout: 60000 }); M.mark('quiz_live_lecturer');
  await sleep(2500);
  // phone: the question arrives, student answers
  await p.locator('.live-poll-panel article.poll').waitFor({ timeout: 30000 }).catch(() => console.log('phone poll not visible')); 
  await sleep(1500); M.mark('quiz_live_phone');
  const opt = p.locator('.live-poll-panel article.poll button').nth(1);
  await tap(opt, 2500); M.mark('phone_answered');
  await sleep(1500); M.mark('lect_results');
  // lecturer closes it
  await click(d, d.locator('.live-poll-panel button:has-text("Close"), .live-poll-panel button:has-text("CLOSE")').first(), 1500).catch(() => console.log('no close button'));
  // lecture ends: audio finished + last captions flushed
  await waitAt(150); M.mark('audio_done');
  await d.evaluate(() => window.scrollTo({ top: 0 })); await sleep(800);
  await click(d, d.locator('button:has-text("END SESSION")'), 2000); M.mark('end_click');
  const confirm = d.getByRole('button', { name: /end session|confirm|yes/i });
  if (await d.locator('[role=dialog] button').count()) { await click(d, d.locator('[role=dialog] button').last(), 2500); M.mark('end_confirmed'); }
  clearInterval(beat);
  await click(d, d.locator('button:has-text("ATTENDANCE")'), 1000);
  await click(d, d.locator('button[aria-label*="efresh"], button:has(svg.lucide-refresh-cw)').first(), 2500); M.mark('attendance_final');
  await d.mouse.wheel(0, 400); await sleep(2500); await d.mouse.wheel(0, -400);
  await click(d, d.locator('button:has-text("ANALYTICS")'), 4000); M.mark('analytics');
  await p.screenshot({ path: __dirname + '/out/phone_end.png' }); await d.screenshot({ path: __dirname + '/out/desk_end.png' });
  const vids = [await d.video().path().catch(() => null), await p.video().path().catch(() => null)];
  M.save({ off, code, vids });
  await D.ctx.close(); await P.ctx.close(); await b.close();
  console.log('done', code, vids);
})().catch(e => { console.error(e); process.exit(1); });
