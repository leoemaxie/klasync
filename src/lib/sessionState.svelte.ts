import type {
  Course,
  Participant,
  RosterStudent,
  Screen,
  Session,
  StudentEnrolledCourse,
} from './types';
import type { AuthUser } from './api/auth';

export class SessionState {
  screen = $state<Screen>('home');
  currentUser = $state<AuthUser | null>(null);
  authNotice = $state('');
  lecturerName = $state('');
  lecturerEmail = $state('');

  // Multi-course & calendar state
  courses = $state<Course[]>([]);
  activeCourse = $state<Course | null>(null);
  academicSession = $state('2025/2026');
  semester = $state('Second Semester');
  studentCourses = $state<StudentEnrolledCourse[]>([]);

  courseId = $state('');
  courseCode = $state('');
  courseTitle = $state('');
  rosterText = $state('');
  roster = $state<RosterStudent[]>([]);
  session = $state<Session | null>(null);
  sessionCode = $state('');
  matric = $state('');
  displayName = $state('');
  joinError = $state('');
  joinedParticipant = $state<Participant | null>(null);
  captions = $state<string[]>([]);
  captionIndex = $state(0);
  captionDraft = $state('');
  copied = $state(false);
  accountCreated = $state(false);
  rosterNotice = $state('');
  apiNotice = $state('');
  isSaving = $state(false);
}

export function createSessionState(): SessionState {
  return new SessionState();
}
