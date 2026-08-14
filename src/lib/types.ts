export type RosterStudent = {
  matric: string;
  name: string;
};

export type Participant = RosterStudent & {
  id?: string;
  verified: boolean;
  joinedAt: string;
  heartbeats: number;
};

export type Course = {
  id: string;
  lecturer_id?: string;
  code: string;
  title: string;
  academic_session: string;
  semester: string;
  is_active?: boolean;
  roster_count?: number;
  session_count?: number;
  last_session_at?: string;
};

export type StudentEnrolledCourse = {
  id: string;
  code: string;
  title: string;
  academic_session: string;
  semester: string;
  lecturer_name: string;
  session_count: number;
  enrolled_at: string;
};

export type Session = {
  id?: string;
  course_id?: string;
  title: string;
  code: string;
  live: boolean;
  createdAt: string;
  participants: Participant[];
};

export type Screen =
  | 'home'
  | 'lecturer'
  | 'lecturer-login'
  | 'lecturer-register'
  | 'student-login'
  | 'student-register'
  | 'recover-password'
  | 'reset-password'
  | 'join'
  | 'live'
  | 'archive'
  | 'not-found';
