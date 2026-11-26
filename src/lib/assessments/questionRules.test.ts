import { describe, expect, it } from 'vitest';
import type { Question } from '$lib/api/assessments';
import {
  fromLocalInput,
  nextOptionId,
  publishProblems,
  questionProblem,
  rubricTotal,
  toLocalInput,
} from './questionRules';

const base: Question = {
  id: 'q',
  assessment_id: 'a',
  position: 0,
  question_type: 'mcq_single',
  prompt_md: 'Which law?',
  points: 2,
  difficulty: 'medium',
  topic_tag: null,
  chapter_index: null,
  public_payload: {
    options: [
      { id: 'a', text: 'Fourier' },
      { id: 'b', text: 'Newton' },
    ],
  },
  key_payload: { correct: ['a'] },
  explanation_md: '',
  origin: 'lecturer',
  source_ranges: [],
  low_confidence: false,
  locked: false,
};

const q = (over: Partial<Question>): Question => ({ ...base, ...over });

describe('questionProblem', () => {
  it('accepts a complete single-choice question', () => {
    expect(questionProblem(base)).toBeNull();
  });

  it('names what is missing, in the order a lecturer would fix it', () => {
    expect(questionProblem(q({ prompt_md: '  ' }))).toMatch(/question text/);
    expect(
      questionProblem(
        q({ public_payload: { options: [{ id: 'a', text: 'x' }] } })
      )
    ).toMatch(/two options/);
    expect(
      questionProblem(
        q({
          public_payload: {
            options: [
              { id: 'a', text: 'x' },
              { id: 'b', text: ' ' },
            ],
          },
        })
      )
    ).toMatch(/every option/);
    expect(questionProblem(q({ key_payload: { correct: [] } }))).toMatch(
      /Mark the correct/
    );
    expect(
      questionProblem(q({ key_payload: { correct: ['a', 'b'] } }))
    ).toMatch(/exactly one/);
    expect(questionProblem(q({ key_payload: { correct: ['z'] } }))).toMatch(
      /one of the options/
    );
  });

  it('allows several correct options for multiple choice', () => {
    expect(
      questionProblem(
        q({ question_type: 'mcq_multi', key_payload: { correct: ['a', 'b'] } })
      )
    ).toBeNull();
  });

  it('checks true/false, short and long answers', () => {
    expect(
      questionProblem(q({ question_type: 'true_false', key_payload: {} }))
    ).toMatch(/true or false/);
    expect(
      questionProblem(
        q({ question_type: 'true_false', key_payload: { correct: false } })
      )
    ).toBeNull();
    expect(
      questionProblem(
        q({
          question_type: 'short_answer',
          key_payload: { accepted: ['k', ''] },
        })
      )
    ).toMatch(/cannot be empty/);
    expect(
      questionProblem(
        q({
          question_type: 'long_answer',
          points: 4,
          key_payload: { rubric: [{ id: 'r1', criterion: 'Law', points: 3 }] },
        })
      )
    ).toMatch(/adds up to 3/);
    expect(
      questionProblem(
        q({
          question_type: 'long_answer',
          points: 4,
          key_payload: {
            rubric: [
              { id: 'r1', criterion: 'Law', points: 3 },
              { id: 'r2', criterion: 'Sign', points: 1 },
            ],
          },
        })
      )
    ).toBeNull();
  });
});

describe('publishProblems', () => {
  it('needs at least one question and lists each problem by number', () => {
    expect(publishProblems([])).toEqual(['Add at least one question.']);
    const problems = publishProblems([
      base,
      q({ prompt_md: '' }),
      q({ key_payload: { correct: [] } }),
    ]);
    expect(problems).toEqual([
      'Question 2: Add the question text.',
      'Question 3: Mark the correct option.',
    ]);
  });

  it('wants some points overall', () => {
    expect(publishProblems([q({ points: 0 })])).toContain(
      'The questions must be worth some points.'
    );
  });
});

describe('helpers', () => {
  it('picks the next unused option id', () => {
    expect(nextOptionId([])).toBe('a');
    expect(
      nextOptionId([
        { id: 'a', text: '' },
        { id: 'b', text: '' },
      ])
    ).toBe('c');
    expect(
      nextOptionId([
        { id: 'a', text: '' },
        { id: 'c', text: '' },
      ])
    ).toBe('b');
  });

  it('adds up a rubric', () => {
    expect(
      rubricTotal([
        { id: 'a', criterion: '', points: 1.5 },
        { id: 'b', criterion: '', points: 2 },
      ])
    ).toBe(3.5);
    expect(rubricTotal(undefined)).toBe(0);
  });

  it('round-trips a local datetime', () => {
    const iso = '2026-03-01T10:30:00.000Z';
    expect(fromLocalInput(toLocalInput(iso))).toBe(iso);
    expect(toLocalInput(null)).toBe('');
    expect(fromLocalInput('')).toBeNull();
    expect(fromLocalInput('not a date')).toBeNull();
  });
});
