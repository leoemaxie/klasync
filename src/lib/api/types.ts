export type ApiRosterStudent = {
  matric_number: string;
  full_name: string;
  email?: string | null;
};

export type ApiSession = {
  id: string;
  title: string;
  short_code: string;
  status: 'live' | 'ended';
};

export type ApiParticipant = {
  id: string;
  matric_number: string;
  display_name: string;
  verification_status: 'verified' | 'provisional';
  heartbeat_count: number;
};

export type ApiCaption = {
  id: string;
  text: string;
  created_at: string;
};

export type SuccessResponse = {
  success: boolean;
  message?: string;
};

export type CountResponse = {
  count: number;
};

export type CreateSessionResponse = {
  session: ApiSession;
  join_url: string;
  qr_payload: string;
};

export type CreateCourseResponse = {
  id: string;
};

export type SessionLookupResponse = {
  session: ApiSession;
  participant_count: number;
};

export type AttendanceSummaryResponse = {
  total: number;
  verified: number;
  provisional: number;
};

export type InviteLookupResponse = {
  session: ApiSession;
  course_code: string;
};

export type RevokeInviteResponse = {
  revoked: boolean;
};
