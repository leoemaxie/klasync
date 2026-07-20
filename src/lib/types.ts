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

export type Screen = "home" | "lecturer" | "join" | "live" | "archive";
