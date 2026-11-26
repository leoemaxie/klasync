import {
  fetchSessionQuiz,
  generateSessionQuiz,
  runStudyJob,
  type SessionQuizQuestion,
} from '$lib/api/aiStudy';

/**
 * One student's pass through a lecture's quiz: load/generate questions, answer
 * them one at a time (answers lock once picked, with the explanation shown),
 * and score the run. Questions persist server-side; the run itself is local.
 */
export class QuizState {
  readonly sessionId: string;

  questions = $state<SessionQuizQuestion[]>([]);
  status = $state<'loading' | 'ready'>('loading');
  generating = $state(false);
  error = $state('');
  index = $state(0);
  picked = $state<number | null>(null);
  score = $state(0);
  finished = $state(false);

  constructor(sessionId: string) {
    this.sessionId = sessionId;
  }

  current = $derived(this.questions[this.index] ?? null);

  async load() {
    this.status = 'loading';
    this.error = '';
    try {
      if (this.sessionId)
        this.questions = await fetchSessionQuiz(this.sessionId);
    } catch (error) {
      this.error =
        error instanceof Error ? error.message : 'Unable to load the quiz.';
    } finally {
      this.status = 'ready';
    }
  }

  async generate(topic?: string) {
    if (this.generating) return;
    this.generating = true;
    this.error = '';
    const before = this.questions.length;
    try {
      await runStudyJob(() => generateSessionQuiz(this.sessionId, topic));
      const all = await fetchSessionQuiz(this.sessionId);
      if (!all.length) {
        this.error = 'No questions were produced. Please try again.';
        return;
      }
      this.questions = all;
      // Continue from the first new question when adding to a finished run.
      if (this.finished || before === 0) {
        this.index = Math.min(before, all.length - 1);
        this.finished = false;
        this.picked = null;
        if (before === 0) this.score = 0;
      }
    } catch (error) {
      this.error =
        error instanceof Error ? error.message : 'Quiz generation failed.';
    } finally {
      this.generating = false;
    }
  }

  answer(choice: number) {
    if (this.picked !== null || !this.current) return;
    this.picked = choice;
    if (choice === this.current.correct_index) this.score += 1;
  }

  next() {
    if (this.picked === null) return;
    this.picked = null;
    if (this.index + 1 >= this.questions.length) this.finished = true;
    else this.index += 1;
  }

  restart() {
    this.index = 0;
    this.picked = null;
    this.score = 0;
    this.finished = false;
  }
}
