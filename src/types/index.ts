export interface Device {
  serial: string;
  status: string;
  model: string | null;
}

export interface SessionInfo {
  id: string;
  device_serial: string;
  app_package: string;
  display_spec: string;
  width: number;
  height: number;
  status: SessionStatus;
}

export type SessionStatus =
  | "Starting"
  | "Running"
  | "Stopped"
  | { Error: string };
