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

export type Session = {
  title: string;
  code: string;
  live: boolean;
  createdAt: string;
  participants: Participant[];
};

export type Screen =
  | "home"
  | "lecturer"
  | "lecturer-login"
  | "lecturer-register"
  | "student-login"
  | "student-register"
  | "recover-password"
  | "reset-password"
  | "join"
  | "live"
  | "archive"
  | "not-found";
