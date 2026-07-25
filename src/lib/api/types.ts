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
